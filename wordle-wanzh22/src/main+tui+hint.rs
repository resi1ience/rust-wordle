use colored::Colorize;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{
    self,
    event::{self, Event, KeyCode},
    execute,
};
use rand::seq::SliceRandom;
use rand::SeedableRng;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::io::{self, Read, Write};

use tui::backend::CrosstermBackend;
use tui::layout::{Constraint, Direction, Layout};
use tui::widgets::{Block, Borders, Paragraph, Wrap};
use tui::{self, Terminal};

mod builtin_words;

// Color: 0:Black, 1:Red, 2:Yellow, 3:Green

/// The main function for the Wordle game, implement your own logic here
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // tty mode instructions

    let is_tty = atty::is(atty::Stream::Stdout);

    // tui init
    let stdout: io::Stdout = io::stdout();
    let backend: CrosstermBackend<&io::Stdout> = CrosstermBackend::new(&stdout);
    let mut terminal: Terminal<CrosstermBackend<&io::Stdout>> = Terminal::new(backend)?;

    let mut block_width: u16 = 0;

    if is_tty {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;

        terminal.clear()?;

        terminal.show_cursor()?;

        let mut input = String::new();

        loop {
            terminal.draw(|f| {
                let size = f.size();
                let input_prompt = Paragraph::new(format!("Your Name：{}", input))
                    .block(Block::default().borders(Borders::ALL))
                    .wrap(Wrap { trim: false });

                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Percentage(75), Constraint::Percentage(25)])
                    .split(size);

                f.render_widget(input_prompt, chunks[1]);
            })?;

            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char(c) => {
                        input.push(c);
                    }
                    KeyCode::Enter => {
                        break;
                    }
                    KeyCode::Backspace => {
                        input.pop();
                    }
                    _ => {}
                }
            }
        }
        terminal.set_cursor(0, 0)?;
        println!("");
        println!("Welcome to wordle, {}!\n", input.trim());
        std::thread::sleep(std::time::Duration::from_millis(2000));
        terminal.clear()?;

        io::stdout().flush().unwrap();
    }

    // command line arguments execution

    // ? add arg :"-h" // "--hint" for answer hint

    let mut answer_designated = false;
    let mut random_mode = false;
    let mut difficult_mode = false;
    let mut stats_mode = false;
    let mut seed_designated = false;
    let mut final_designated = false;
    let mut acceptable_designated = false;
    let mut game_state_save = false;
    let mut set_config_mode = false;
    let mut hint_mode = false;

    let mut day = 1;
    let mut seed: u64 = 20230701;
    let mut final_filename = String::new();
    let mut acceptable_filename = String::new();
    let mut save_filename = String::new();
    let mut config_file_name = String::new();
    let final_word_list_use: Vec<String>;
    let acceptable_word_list_use: Vec<String>;

    let mut answer = String::from("");

    let args: Vec<String> = std::env::args().collect();

    for (index, arg) in args.iter().enumerate() {
        if arg == "-c" || arg == "--config" {
            set_config_mode = true;
            if let Some(next_arg) = args.get(index + 1) {
                config_file_name = next_arg.trim().to_string();
            }
        }
    }

    // config mode init by config

    if set_config_mode == true {
        if let Ok(mut config_file) = std::fs::File::open(&config_file_name) {
            let mut config_content = String::new();
            config_file.read_to_string(&mut config_content)?;
            let config_data: Value = serde_json::from_str(&config_content)?;

            if let Value::Object(_) = config_data {
                ()
            } else {
                panic!("The JSON contains invalid data");
            }

            if let Value::Object(obj) = config_data {
                for (key, val) in obj.iter() {
                    if key == "random" {
                        random_mode = val.as_bool().unwrap();
                    } else if key == "difficult" {
                        difficult_mode = val.as_bool().unwrap();
                    } else if key == "stats" {
                        stats_mode = val.as_bool().unwrap();
                    } else if key == "day" {
                        day = val.as_i64().unwrap() as i32;
                        if answer_designated == true {
                            panic!("Can't use -d/--day in Specify Answer Mode!");
                        }
                    } else if key == "seed" {
                        seed_designated = true;
                        seed = val.as_u64().unwrap();
                        if answer_designated == true {
                            panic!("Can't use -s/--seed in Specify Answer Mode!");
                        }
                    } else if key == "final_set" {
                        final_designated = true;
                        final_filename = val.as_str().unwrap().trim().to_string();
                    } else if key == "acceptable_set" {
                        acceptable_designated = true;
                        acceptable_filename = val.as_str().unwrap().trim().to_string();
                    } else if key == "state" {
                        game_state_save = true;
                        save_filename = val.as_str().unwrap().trim().to_string();
                    } else if key == "word" {
                        answer_designated = true;
                        answer = val.as_str().unwrap().trim().to_string();
                        if random_mode == true {
                            panic!("Can't use -w/--word in Random Answer Mode!");
                        }
                    } else {
                        panic!("The JSON contains invalid key");
                    }
                }
            }
        } else {
            panic!("Can't find config file");
        }
    }

    // other args in arg line

    for (index, arg) in args.iter().enumerate() {
        if arg == "-r" || arg == "--random" {
            random_mode = true;
        }

        if arg == "-D" || arg == "--difficult" {
            difficult_mode = true;
        }

        if arg == "-t" || arg == "--stats" {
            stats_mode = true;
        }

        if arg == "-f" || arg == "--final-set" {
            final_designated = true;
            if let Some(next_arg) = args.get(index + 1) {
                final_filename = next_arg.trim().to_string();
            }
        }

        if arg == "-a" || arg == "--acceptable-set" {
            acceptable_designated = true;
            if let Some(next_arg) = args.get(index + 1) {
                acceptable_filename = next_arg.trim().to_string();
            }
        }

        if arg == "-h" || arg == "--hint" {
            hint_mode = true;
        }
    }

    // args with conflicts execution

    for (index, arg) in args.iter().enumerate() {
        if arg == "-d" || arg == "--day" {
            if random_mode == true {
                if let Some(next_arg) = args.get(index + 1) {
                    day = next_arg.trim().to_string().parse::<i32>().unwrap();
                }
            } else if answer_designated == true {
                panic!("Can't use -d/--day in Specify Answer Mode!");
            }
        }

        if arg == "-s" || arg == "--seed" {
            if random_mode == true {
                seed_designated = true;
                if let Some(next_arg) = args.get(index + 1) {
                    seed = next_arg.trim().to_string().parse::<u64>().unwrap();
                }
            } else if answer_designated == true {
                panic!("Can't use -s/--seed in Specify Answer Mode!");
            }
        }

        if arg == "-w" || arg == "--word" {
            answer_designated = true;
            if random_mode == true {
                panic!("Can't use -w/--word in Random Answer Mode!");
            }
        }

        if arg == "-S" || arg == "--state" {
            if random_mode == true {
                game_state_save = true;
                if let Some(next_arg) = args.get(index + 1) {
                    save_filename = next_arg.trim().to_string();
                }
            }
        }
    }

    if random_mode == true && answer_designated == true {
        panic!("random mode and answer designated mode conflicted!");
    }

    // arg read end, arg init begin

    // f mode and a mode init by file

    let mut final_word_list: Vec<String> = Vec::new();
    let mut final_word_list_set: HashSet<String> = HashSet::new(); // for deduplication
    let mut acceptable_word_list: Vec<String> = Vec::new();
    let mut acceptable_word_list_set: HashSet<String> = HashSet::new(); // for deduplication

    if acceptable_designated == true {
        if let Ok(mut acceptable_file) = std::fs::File::open(acceptable_filename) {
            let mut acceptable_string = String::new();
            acceptable_file.read_to_string(&mut acceptable_string)?;
            for word in acceptable_string.lines() {
                if word.chars().all(|c| c.is_alphabetic()) == false || word.len() != 5 {
                    panic!("The Word Type Doesn't Meet Need");
                }

                if acceptable_word_list_set.contains(&word.to_ascii_uppercase()) == true {
                    panic!("Same Word Appeared In The List");
                } else {
                    acceptable_word_list.push(word.to_ascii_uppercase());
                    acceptable_word_list_set.insert(word.to_ascii_uppercase());
                }
            }
            acceptable_word_list.sort();
            acceptable_word_list_use = acceptable_word_list;
        } else {
            panic!("Can't Find Acceptable File");
        }
    } else {
        acceptable_word_list_use = (*builtin_words::ACCEPTABLE)
            .to_vec()
            .iter()
            .map(|x| x.to_string())
            .collect();
    }

    if final_designated == true {
        if let Ok(mut final_file) = std::fs::File::open(final_filename) {
            let mut final_string = String::new();
            final_file.read_to_string(&mut final_string)?;
            for word in final_string.lines() {
                if word.chars().all(|c| c.is_alphabetic()) == false || word.len() != 5 {
                    panic!("The Word Type Doesn't Meet Need");
                }

                if final_word_list_set.contains(&word.to_ascii_uppercase()) == true {
                    panic!("Same Word Appeared In The List");
                } else if acceptable_word_list_set.contains(&word.to_ascii_uppercase()) == false {
                    panic!("The Final Word Doesn't Appear In Acceptable Word List");
                } else {
                    final_word_list.push(word.to_ascii_uppercase());
                    final_word_list_set.insert(word.to_ascii_uppercase());
                }
            }
            final_word_list.sort();
            final_word_list_use = final_word_list;
        } else {
            panic!("Can't Find Final File");
        }
    } else {
        final_word_list_use = (*builtin_words::FINAL)
            .to_vec()
            .iter()
            .map(|x| x.to_string())
            .collect();
    }

    // random mode init by seed

    let mut final_index: Vec<i32> = Vec::new();
    let final_len: usize = final_word_list_use.len();

    for i in 0..final_len {
        final_index.push(i as i32);
    }

    if seed_designated == true {
        let mut rand_num = rand::rngs::StdRng::seed_from_u64(seed);
        final_index.shuffle(&mut rand_num);
    }

    // judge if day is too big

    if day > final_len as i32 {
        panic!("day > final len");
    }

    // judge if the designated ans in wordlist or not

    for (index, arg) in args.iter().enumerate() {
        if arg == "-w" || arg == "--word" {
            answer_designated = true;
            if let Some(next_arg) = args.get(index + 1) {
                answer = next_arg.trim().to_string();
            }
        }
    }

    if answer_designated == true {
        if final_word_list_use
            .iter()
            .find(|&s| &s.to_ascii_lowercase() == &answer)
            == None
        {
            panic!("The Answer is not in The Final List");
        }
    }

    // init game state if save-mode on

    let mut total_rounds = 0;
    let mut game: Vec<(String, Vec<String>)> = Vec::new(); // (a, b) : a:answer, b:guesses, index: rounds
    let mut save_win_num = 0;
    let mut save_win_try_sum = 0;
    let mut save_guess_word_record: HashMap<String, i32> = HashMap::new();

    if game_state_save == true {
        if let Ok(mut json_file) = std::fs::File::open(&save_filename) {
            let mut json_content = String::new();
            json_file.read_to_string(&mut json_content)?;
            let json_data: Value = serde_json::from_str(&json_content)?;

            // check if data consists of key-value

            if let Value::Object(_) = json_data {
            } else {
                panic!("The JSON contains invalid data");
            }

            let mut total_rounds_exist = false;
            let mut game_exist = false;

            // find total_rounds and game

            if let Some(val) = json_data.get("total_rounds") {
                total_rounds_exist = true;
                total_rounds = val.as_i64().unwrap();
            }

            if let Value::Object(obj) = json_data {
                for (key, _value) in &obj {
                    if key != "games" && key != "total_rounds" {
                        panic!("The JSON contains invalid key");
                    }
                }
                if let Some(games_val) = obj.get("games") {
                    game_exist = true;
                    if let Value::Array(game_arr) = games_val {
                        for game_value in game_arr {
                            if let Value::Object(game_obj) = game_value {
                                if let (Some(answer), Some(guesses)) = (
                                    game_obj.get("answer").and_then(Value::as_str),
                                    game_obj.get("guesses").and_then(Value::as_array),
                                ) {
                                    let mut inner_vec: Vec<String> = Vec::new();
                                    for item in guesses {
                                        if let Value::String(guess) = item {
                                            inner_vec.push(guess.clone());
                                            let count = save_guess_word_record
                                                .entry(guess.clone())
                                                .or_insert(0);
                                            *count += 1;
                                        }
                                    }
                                    if inner_vec.contains(&answer.to_string()) == true {
                                        save_win_num += 1;
                                        save_win_try_sum += inner_vec.len()
                                    }
                                    game.push((answer.to_string().clone(), inner_vec));
                                }
                            }
                        }
                    } else {
                        panic!("Unexpected Type");
                    }
                }
            }

            if total_rounds_exist == true && game_exist == true {
                if total_rounds as usize != game.len() {
                    panic!("total rounds != game.len");
                }
            } else if total_rounds_exist == false && game_exist == true {
                total_rounds = game.len() as i64;
            }
        } else {
            ()
        }
    }

    // game start

    let mut game_num = total_rounds + 1;
    let mut win_num = save_win_num;
    let mut guess_win_try_sum = save_win_try_sum;
    let mut guess_word_record = save_guess_word_record;

    loop {
        // init alphabet which record state of each letter

        let mut current_game: (String, Vec<String>) = (String::new(), Vec::new());

        let mut alphabet: HashMap<String, i32> = HashMap::new();
        for i in 97..123
        // from a->z
        {
            alphabet.insert(((i as u8) as char).to_string(), 0);
        }

        // answer input

        if answer_designated == false && random_mode == false {
            // answer input

            let mut answer_untrimed = String::new();

            if is_tty {
                terminal.clear()?;

                loop {
                    terminal.draw(|f| {
                        let size = f.size();
                        block_width = size.height;
                        let input_prompt =
                            Paragraph::new(format!("Please Enter The Answer：{}", answer_untrimed))
                                .block(Block::default().borders(Borders::ALL))
                                .wrap(Wrap { trim: false });

                        let chunks = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
                            .split(size);

                        f.render_widget(input_prompt, chunks[0]);
                    })?;

                    if let Event::Key(key) = event::read()? {
                        match key.code {
                            KeyCode::Char(c) => {
                                answer_untrimed.push(c);
                            }
                            KeyCode::Enter => {
                                break;
                            }
                            KeyCode::Backspace => {
                                answer_untrimed.pop();
                            }
                            _ => {}
                        }
                    }
                }
            } else {
                io::stdin().read_line(&mut answer_untrimed)?;
            }

            answer = answer_untrimed.trim().to_string();

            if final_word_list_use
                .iter()
                .find(|&s| &s.to_ascii_lowercase() == &answer)
                == None
            {
                panic!("The Input Is Not In the Answer List");
            }
        }

        // answer generate if random mode on

        if random_mode == true {
            let random_index = final_index[(day - 1) as usize];

            answer = final_word_list_use[random_index as usize].to_string();
        }

        // guess begin

        current_game.0 = answer.clone().to_ascii_uppercase(); // save current_game answer
        answer = answer.to_ascii_lowercase(); // use lowercase to judge

        let mut guess_time = 1;
        let mut win_or_not = false;
        let mut previous_guess_string = String::from("");

        // (a, b, c) : a=letter, b=index, c=state
        let mut previous_guess_for_difficult: Vec<(char, i32, i32)> = Vec::new();
        let mut yellow_letter_in_previous_guess_max_count: HashMap<char, i32> = HashMap::new();

        while guess_time <= 6 {
            // hint generate if hint_mode on

            if hint_mode == true && guess_time >= 2 {
                let cursor_y = terminal.get_cursor().unwrap().1;
                terminal.set_cursor(1, cursor_y)?;

                println!("\nPossible Answers:");

                let mut yellow_letter_in_new_guess_count_hint: HashMap<char, i32> = HashMap::new();
                let mut all_possible_answers: Vec<String> = Vec::new();

                for word in &acceptable_word_list_use {
                    let mut validity = true;

                    // current_game.1 : current game previous guesses

                    if current_game.1.contains(&word.to_ascii_uppercase()) == true {
                        continue;
                    }

                    let word_vec: Vec<char> = word.chars().collect(); // switch string to vec

                    for guess_index in 0..5 {
                        let count = yellow_letter_in_new_guess_count_hint
                            .entry(word_vec[guess_index])
                            .or_insert(0);
                        *count += 1;
                    }

                    let mut previous_guess_letter_count: HashMap<char, i32> = HashMap::new();

                    let mut index_flag = 0;

                    let mut red_letter_save: Vec<char> = Vec::new();
                    let mut other_letter_save: Vec<char> = Vec::new();

                    for (letter, index, state) in &previous_guess_for_difficult {
                        let count = previous_guess_letter_count.entry(*letter).or_insert(0);
                        *count += 1;

                        if state == &(3 as i32) {
                            other_letter_save.push(*letter);
                            if &word_vec[*index as usize] != letter {
                                validity = false;
                                break;
                            }
                        } else if state == &(2 as i32) {
                            other_letter_save.push(*letter);
                            if word_vec.contains(letter) == false {
                                validity = false;
                                break;
                            }

                            if &word_vec[*index as usize] == letter {
                                validity = false;
                                break;
                            }

                            match yellow_letter_in_new_guess_count_hint.get(letter) {
                                None => (),
                                Some(x)
                                    if x < yellow_letter_in_previous_guess_max_count
                                        .get(letter)
                                        .unwrap() =>
                                {
                                    validity = false
                                }
                                Some(_) => (),
                            };
                            if validity == false {
                                break;
                            }
                        } else if state == &(1 as i32) {
                            red_letter_save.push(*letter);
                        }

                        index_flag += 1;
                        if index_flag == 5 {
                            for letter in &red_letter_save {
                                if other_letter_save.contains(letter) == true {
                                    let mut new_guess_letter_appear_times = 0;
                                    for new_index in 0..5 {
                                        if &word_vec[new_index as usize] == letter {
                                            new_guess_letter_appear_times += 1;
                                        }
                                    }
                                    if &new_guess_letter_appear_times
                                        >= previous_guess_letter_count.get(letter).unwrap()
                                    {
                                        validity = false;
                                        break;
                                    }
                                } else {
                                    let mut new_guess_letter_appear_times = 0;
                                    for new_index in 0..5 {
                                        if &word_vec[new_index as usize] == letter {
                                            new_guess_letter_appear_times += 1;
                                        }
                                    }
                                    if new_guess_letter_appear_times > 0 {
                                        validity = false;
                                        break;
                                    }
                                }
                            }

                            red_letter_save.clear();
                            other_letter_save.clear();
                            previous_guess_letter_count.clear();
                            index_flag = 0;
                        }
                    }

                    if validity == true {
                        all_possible_answers.push(word.clone().to_string());
                    }
                }

                // calculate entropy

                if all_possible_answers.len() > 1 {
                    let mut possible_word_list: Vec<(String, f64)> = all_possible_answers
                        .par_iter()
                        .map(|word| {
                            // enum word state
                            let mut entropy: f64 = 0.0;
                            let possible_answer_num = all_possible_answers.len() - 1;

                            let mut current_guess_letter_count: HashMap<char, i32> = HashMap::new();
                            for index in 0..5 {
                                let count = current_guess_letter_count
                                    .entry(word.chars().nth(index).unwrap())
                                    .or_insert(0);
                                *count += 1;
                            }

                            for state in 0..244 {
                                let mut cur_state_possible_answer_num = 0;

                                let mut letter_state: Vec<i32> = Vec::new();
                                let mut x = state;
                                for _i in 0..5 {
                                    letter_state.push(x % 3 + 1);
                                    x = x / 3;
                                }

                                let mut temp_yellow_letter_count: HashMap<char, i32> =
                                    HashMap::new();

                                let mut other_save: Vec<char> = Vec::new();

                                for index in 0..5 {
                                    if letter_state[index] == 2 {
                                        let count_tmp = temp_yellow_letter_count
                                            .entry(word.chars().nth(index).unwrap())
                                            .or_insert(0);
                                        *count_tmp += 1;
                                    }
                                    if letter_state[index] != 1 {
                                        other_save.push(word.chars().nth(index).unwrap());
                                    }
                                }

                                let mut validity: bool;

                                for other_word in &all_possible_answers {
                                    validity = true;
                                    for index in 0..5 {
                                        if letter_state[index] == 3 {
                                            if word.chars().nth(index)
                                                != other_word.chars().nth(index)
                                            {
                                                validity = false;
                                            }
                                        } else if letter_state[index] == 1 {
                                            if other_save
                                                .contains(&word.chars().nth(index).unwrap())
                                            {
                                                let mut new_guess_letter_appear_times = 0;
                                                for new_index in 0..5 {
                                                    if other_word.chars().nth(new_index)
                                                        == word.chars().nth(index)
                                                    {
                                                        new_guess_letter_appear_times += 1;
                                                    }
                                                }
                                                if &new_guess_letter_appear_times
                                                    >= current_guess_letter_count
                                                        .get(&word.chars().nth(index).unwrap())
                                                        .unwrap()
                                                {
                                                    validity = false;
                                                    break;
                                                }
                                            } else {
                                                let mut new_guess_letter_appear_times = 0;
                                                for new_index in 0..5 {
                                                    if other_word.chars().nth(new_index)
                                                        == word.chars().nth(index)
                                                    {
                                                        new_guess_letter_appear_times += 1;
                                                    }
                                                }
                                                if new_guess_letter_appear_times > 0 {
                                                    validity = false;
                                                    break;
                                                }
                                            }
                                        } else if letter_state[index] == 2 {
                                            if other_word.contains(word.chars().nth(index).unwrap())
                                                == false
                                            {
                                                validity = false;
                                                break;
                                            }

                                            if other_word.chars().nth(index).unwrap()
                                                == word.chars().nth(index).unwrap()
                                            {
                                                validity = false;
                                                break;
                                            }

                                            match temp_yellow_letter_count
                                                .get(&word.chars().nth(index).unwrap())
                                            {
                                                None => (),
                                                Some(x)
                                                    if x < temp_yellow_letter_count
                                                        .get(&word.chars().nth(index).unwrap())
                                                        .unwrap() =>
                                                {
                                                    validity = false
                                                }
                                                Some(_) => (),
                                            };
                                            if validity == false {
                                                break;
                                            }
                                        }
                                    }
                                    if validity == true {
                                        cur_state_possible_answer_num += 1;
                                    }
                                }

                                if cur_state_possible_answer_num != 0 {
                                    let p: f64 = (cur_state_possible_answer_num as f64)
                                        / (possible_answer_num as f64);

                                    entropy += p * (-f64::log2(p));
                                }
                            }
                            (word.to_ascii_uppercase(), entropy)
                        })
                        .collect();

                    possible_word_list.sort_by(|a, b| a.0.cmp(&b.0));
                    possible_word_list.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

                    let cursor_y = terminal.get_cursor().unwrap().1;
                    terminal.set_cursor(1, cursor_y)?;

                    for i in 0..std::cmp::min(10, possible_word_list.len()) {
                        print!(
                            "{} {:.3} ",
                            possible_word_list[i].0, possible_word_list[i].1
                        );
                    }
                } else {
                    let cursor_y = terminal.get_cursor().unwrap().1;
                    terminal.set_cursor(1, cursor_y)?;
                    println!("{} 0", all_possible_answers[0].to_ascii_uppercase());
                }

                println!("");
            }

            // guess input

            let mut guess_untrimed = String::new();

            if is_tty {
                let cursor_y = terminal.get_cursor().unwrap().1;
                terminal.set_cursor(0, cursor_y)?;

                loop {
                    terminal.draw(|f| {
                        let size = f.size();
                        let input_prompt =
                            Paragraph::new(format!("Please Enter Your Guess：{}", guess_untrimed))
                                .block(Block::default().borders(Borders::ALL))
                                .wrap(Wrap { trim: false });

                        let chunks = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
                            .split(size);

                        f.render_widget(input_prompt, chunks[0]);
                    })?;

                    if let Event::Key(key) = event::read()? {
                        match key.code {
                            KeyCode::Char(c) => {
                                guess_untrimed.push(c);
                            }
                            KeyCode::Enter => {
                                break;
                            }
                            KeyCode::Backspace => {
                                guess_untrimed.pop();
                            }
                            _ => {}
                        }
                    }
                }

                terminal.clear()?;
            } else {
                io::stdin().read_line(&mut guess_untrimed)?;
            }

            let guess = guess_untrimed.trim();

            if acceptable_word_list_use
                .iter()
                .find(|&s| &s.to_ascii_lowercase() == &guess)
                == None
            {
                if is_tty {
                    terminal.set_cursor(0, 0)?;
                }
                println!("INVALID");
                if is_tty {
                    std::thread::sleep(std::time::Duration::from_millis(1000));
                }
                continue;
            }

            current_game.1.push(guess.clone().to_ascii_uppercase());

            let guess_vec: Vec<char> = guess.chars().collect(); // process guess to more accessible type

            // judge validity for difficult mode

            let mut yellow_letter_in_new_guess_count: HashMap<char, i32> = HashMap::new();
            let mut yellow_letter_in_current_guess_count: HashMap<char, i32> = HashMap::new();

            if difficult_mode == true {
                let mut validity = true;

                for guess_index in 0..5 {
                    let count = yellow_letter_in_new_guess_count
                        .entry(guess_vec[guess_index])
                        .or_insert(0);
                    *count += 1;
                }

                for (letter, index, state) in &previous_guess_for_difficult {
                    if state == &(3 as i32) {
                        if &guess_vec[*index as usize] != letter {
                            validity = false;
                            break;
                        }
                    } else if state == &(2 as i32) {
                        if guess_vec.contains(letter) == false {
                            validity = false;
                            break;
                        }

                        match yellow_letter_in_new_guess_count.get(letter) {
                            None => (),
                            Some(x)
                                if x < yellow_letter_in_previous_guess_max_count
                                    .get(letter)
                                    .unwrap() =>
                            {
                                validity = false
                            }
                            Some(_) => (),
                        };
                        if validity == false {
                            break;
                        }
                    }
                }

                if validity == false {
                    if is_tty {
                        terminal.set_cursor(0, 0)?;
                    }
                    println!("INVALID");
                    if is_tty {
                        std::thread::sleep(std::time::Duration::from_millis(1000));
                    }
                    continue;
                }
            }

            // save guess word

            let count = guess_word_record
                .entry(guess.to_string().to_ascii_uppercase())
                .or_insert(0);
            *count += 1;

            //

            let guess_print = guess.clone();

            // process answer to more accessible type

            let answer_vec: Vec<char> = answer.chars().collect();
            let guess_print_vec: Vec<char> = guess_print.chars().collect();

            let mut match_guess_vec: Vec<bool> = vec![false; 5]; // according to guess
            let mut match_answer_vec: Vec<bool> = vec![false; 5]; // according to answer

            let mut right_letter_count = 0;

            // match the Green letter first, if exist

            for index in 0..5 {
                if guess_vec[index] == answer_vec[index] {
                    alphabet.insert(guess_vec[index].to_string(), 3);
                    match_answer_vec[index] = true;
                    match_guess_vec[index] = true;
                    right_letter_count += 1;
                }
            }

            // match other letters

            for guess_index in 0..5 {
                // previously correctly matched

                if match_guess_vec[guess_index] == true {
                    if is_tty {
                        previous_guess_string = format!(
                            "{}{}",
                            previous_guess_string.clone(),
                            guess_print_vec[guess_index].to_string().green()
                        );
                    } else {
                        print!("G");
                    }
                    previous_guess_for_difficult.push((
                        guess_vec[guess_index],
                        guess_index as i32,
                        3 as i32,
                    ));

                    continue;
                }

                // unmatched

                let mut letter_matched = false;

                for answer_index in 0..5 {
                    if match_answer_vec[answer_index] == true {
                        continue;
                    }
                    // correct letter but incorrect place
                    if guess_vec[guess_index] == answer_vec[answer_index] {
                        if alphabet.get(&guess_vec[guess_index].to_string()) < Some(&2) {
                            alphabet.insert(guess_vec[guess_index].to_string(), 2);
                        }
                        match_answer_vec[answer_index] = true;
                        letter_matched = true;

                        if is_tty {
                            previous_guess_string = format!(
                                "{}{}",
                                previous_guess_string.clone(),
                                guess_print_vec[guess_index].to_string().yellow()
                            );
                        } else {
                            print!("Y");
                        }
                        previous_guess_for_difficult.push((
                            guess_vec[guess_index],
                            guess_index as i32,
                            2 as i32,
                        ));
                        let count = yellow_letter_in_current_guess_count
                            .entry(guess_vec[guess_index])
                            .or_insert(0);
                        *count += 1;
                        match yellow_letter_in_previous_guess_max_count.get(&guess_vec[guess_index])
                        {
                            Some(x) if x < count => yellow_letter_in_previous_guess_max_count
                                .insert(guess_vec[guess_index], *count),
                            None => yellow_letter_in_previous_guess_max_count
                                .insert(guess_vec[guess_index], *count),
                            Some(_) => None,
                        };
                        break;
                    }
                }

                // still unmatched

                if letter_matched == false {
                    if alphabet.get(&guess_vec[guess_index].to_string()) < Some(&1) {
                        alphabet.insert(guess_vec[guess_index].to_string(), 1);
                    }
                    if is_tty {
                        previous_guess_string = format!(
                            "{}{}",
                            previous_guess_string.clone(),
                            guess_print_vec[guess_index].to_string().red()
                        );
                    } else {
                        print!("R");
                    }
                    previous_guess_for_difficult.push((
                        guess_vec[guess_index],
                        guess_index as i32,
                        1 as i32,
                    ));
                }
            }

            // print previous guess if is tty

            if is_tty {
                let cursor_y = terminal.get_cursor().unwrap().1;
                terminal.set_cursor(1, cursor_y + block_width + 1)?;
                println!("");
                println!("Your Guess:");

                let cursor_y = terminal.get_cursor().unwrap().1;
                terminal.set_cursor(1, cursor_y)?;

                println!("{}", previous_guess_string);
                previous_guess_string = format!(
                    "{}{}",
                    previous_guess_string.clone(),
                    "\n".to_string().black()
                );
            } else {
                print!(" ");
            }

            // print alphabet state

            let keyboard: Vec<Vec<char>> = vec![
                vec!['q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p'],
                vec!['a', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l'],
                vec!['z', 'x', 'c', 'v', 'b', 'n', 'm'],
            ];

            if is_tty {
                let cursor_y = terminal.get_cursor().unwrap().1;
                terminal.set_cursor(1, cursor_y)?;
                println!("Alphabet State:");

                let cursor_y = terminal.get_cursor().unwrap().1;
                terminal.set_cursor(1, cursor_y)?;

                for line in keyboard {
                    let cursor_y = terminal.get_cursor().unwrap().1;
                    terminal.set_cursor(1, cursor_y)?;
                    for i in line {
                        match alphabet.get(&i.to_string()) {
                            Some(&0) => print!("{}", (i as u8 as char).to_string()),
                            Some(&1) => print!("{}", (i as u8 as char).to_string().red()),
                            Some(&2) => print!("{}", (i as u8 as char).to_string().yellow()),
                            Some(&3) => print!("{}", (i as u8 as char).to_string().green()),
                            _ => unimplemented!(),
                        }
                    }
                    println!("");
                }

                std::thread::sleep(std::time::Duration::from_millis(2000));
            } else {
                for i in 97..123 {
                    match alphabet.get(&((i as u8) as char).to_string()) {
                        Some(&0) => print!("X"),
                        Some(&1) => print!("R"),
                        Some(&2) => print!("Y"),
                        Some(&3) => print!("G"),
                        _ => unimplemented!(),
                    }
                }
                println!("");
            }

            // win state judge

            if right_letter_count == 5 {
                win_or_not = true;
                break;
            }

            guess_time += 1;
        }

        game.push(current_game);

        // guess end

        if is_tty {
            let cursor_y = terminal.get_cursor().unwrap().1;
            terminal.set_cursor(1, cursor_y)?;
            println!("");
            println!("The Game {} Result:", game_num);
        }

        if win_or_not == true {
            if is_tty {
                let cursor_y = terminal.get_cursor().unwrap().1;
                terminal.set_cursor(1, cursor_y)?;
                let print_line = format!("CORRECT! You Guessed {} Times.", guess_time).green();
                println!("{}", print_line);
                std::thread::sleep(std::time::Duration::from_millis(3000));
            } else {
                println!("CORRECT {}", guess_time);
            }
            guess_win_try_sum += guess_time;
            win_num += 1;
        } else {
            if is_tty {
                let cursor_y = terminal.get_cursor().unwrap().1;
                terminal.set_cursor(1, cursor_y)?;
                let print_line =
                    format!("Failed... The Answer is {}.", answer.to_ascii_uppercase()).red();
                println!("{}", print_line);
                std::thread::sleep(std::time::Duration::from_millis(3000));
            } else {
                println!("FAILED {}", answer.to_ascii_uppercase());
            }
        }

        // print stats if stats_mode on

        if stats_mode == true {
            if is_tty {
                if win_num != 0 {
                    let cursor_y = terminal.get_cursor().unwrap().1;
                    terminal.set_cursor(1, cursor_y)?;
                    println!(
                        "Win count:{}, Lose count:{}, Average Try count: {:.2}",
                        win_num,
                        game_num - win_num,
                        (guess_win_try_sum as f32) / (win_num as f32)
                    );
                } else {
                    let cursor_y = terminal.get_cursor().unwrap().1;
                    terminal.set_cursor(1, cursor_y)?;
                    println!(
                        "Win count:{}, Lose count:{}, Average try count: {:.2}",
                        win_num,
                        game_num - win_num,
                        0.00
                    );
                }
            } else {
                if win_num != 0 {
                    println!(
                        "{} {} {:.2}",
                        win_num,
                        game_num - win_num,
                        (guess_win_try_sum as f32) / (win_num as f32)
                    );
                } else {
                    println!("{} {} {:.2}", win_num, game_num - win_num, 0.00);
                }
            }

            let mut vec_for_sort: Vec<(&String, &i32)> = guess_word_record.iter().collect();

            vec_for_sort.sort_by(|a, b| a.0.cmp(b.0));
            vec_for_sort.sort_by(|a, b| b.1.cmp(a.1));

            if is_tty {
                let cursor_y = terminal.get_cursor().unwrap().1;
                terminal.set_cursor(1, cursor_y)?;
                println!("");
                print!("The Top 5 Words You Guessed Most Frequently:");
            }

            for i in 0..std::cmp::min(5, vec_for_sort.len()) {
                if i == 4 || i == std::cmp::min(5, vec_for_sort.len()) - 1 {
                    print!(
                        "{} {}",
                        vec_for_sort[i].0.to_ascii_uppercase(),
                        vec_for_sort[i].1
                    );
                } else {
                    print!(
                        "{} {} ",
                        vec_for_sort[i].0.to_ascii_uppercase(),
                        vec_for_sort[i].1
                    );
                }
            }
            println!("");
            if is_tty {
                std::thread::sleep(std::time::Duration::from_millis(3000));
            }
        }

        // query for next game

        if answer_designated == true {
            break;
        }

        let mut line_untrimed = String::new();

        if is_tty {
            terminal.clear()?;

            loop {
                terminal.draw(|f| {
                    let size = f.size();
                    let input_prompt = Paragraph::new(format!(
                        "Start A New Game or Not? Y for Yes, N for Not:{}",
                        line_untrimed
                    ))
                    .block(Block::default().borders(Borders::ALL))
                    .wrap(Wrap { trim: false });

                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
                        .split(size);

                    f.render_widget(input_prompt, chunks[0]);
                })?;

                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char(c) => {
                            line_untrimed.push(c);
                        }
                        KeyCode::Enter => {
                            break;
                        }
                        KeyCode::Backspace => {
                            line_untrimed.pop();
                        }
                        _ => {}
                    }
                }
            }

            terminal.clear()?;
        } else {
            io::stdin().read_line(&mut line_untrimed)?;
        }
        let line = line_untrimed.trim();

        total_rounds = game_num;

        if line == String::from("Y") {
            ()
        } else if line == String::from("N") {
            break;
        }

        day += 1;
        game_num += 1;
    }

    // write game_state into save_file

    if game_state_save == true {
        let mut save_file = std::fs::File::create(&save_filename)?;
        save_file.write(b"{\n")?;

        let save_game_num_line = format!("  \"total_rounds\": {},\n", total_rounds);
        save_file.write(save_game_num_line.as_bytes())?;

        let all_game_line = format!("  \"games\": [\n");
        save_file.write(all_game_line.as_bytes())?;

        for i in 0..total_rounds {
            save_file.write("    {\n".as_bytes())?;

            let answer_line = format!("      \"answer\": \"{}\",\n", game[i as usize].0);
            save_file.write(answer_line.as_bytes())?;

            let guess_line = format!("      \"guesses\": {:?}\n", game[i as usize].1);
            save_file.write(guess_line.as_bytes())?;

            save_file.write("    }".as_bytes())?;
            if i != total_rounds - 1 {
                save_file.write(",\n".as_bytes())?;
            } else {
                save_file.write("\n".as_bytes())?;
            }
        }

        save_file.write(b"  ]\n")?;
        save_file.write(b"}")?;
    }

    if is_tty {
        disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen)?;
    }

    // saving game state in a new way to be done

    Ok(())
}
