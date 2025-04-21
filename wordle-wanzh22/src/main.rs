use colored::Colorize;
use console;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::io::{self, Read, Write};
mod builtin_words;

// Color: 0:Black, 1:Red, 2:Yellow, 3:Green

fn print_tty_init_screen() -> Result<(), Box<dyn std::error::Error>> {
    print!("{}", console::style("Your name: ").bold().red());
    io::stdout().flush().unwrap();
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    println!("Welcome to wordle, {}!", line.trim());
    io::stdout().flush().unwrap();
    Ok(())
}

/// The main function for the Wordle game
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // tty mode instructions

    let is_tty = atty::is(atty::Stream::Stdout);

    if is_tty {
        let _ = print_tty_init_screen();
    }

    // ! command line arguments execution

    let mut answer_designated = false;
    let mut random_mode = false;
    let mut difficult_mode = false;
    let mut stats_mode = false;
    let mut seed_designated = false;
    let mut final_designated = false;
    let mut acceptable_designated = false;
    let mut game_state_save = false;
    let mut set_config_mode = false;

    let mut day = 1;
    let mut seed: u64 = 20230701;
    let mut final_filename = String::new();
    let mut acceptable_filename = String::new();
    let mut save_filename = String::new();
    let mut config_file_name = String::new();
    let final_word_list_use: Vec<String>;
    let acceptable_word_list_use: Vec<String>;

    let mut answer = String::from(""); // for -w mode

    let args: Vec<String> = std::env::args().collect(); // read arg

    // * check config mode

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
            let config_data: Value = serde_json::from_str(&config_content)?; // parse config data

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

    // * other args in arg line

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
    }

    // * args with conflicts execution

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
            // only save game state in random mode
            if random_mode == true {
                game_state_save = true;
                if let Some(next_arg) = args.get(index + 1) {
                    save_filename = next_arg.trim().to_string();
                }
            }
        }
    }

    // check if config and arg conflicts here

    if random_mode == true && answer_designated == true {
        panic!("random mode and answer designated mode conflicted!");
    }

    // * arg read end, arg init begin

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
            .map(|x| x.to_string().to_ascii_uppercase())
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
                    // check if final word is in the acceptable list
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
            .map(|x| x.to_string().to_ascii_uppercase())
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

    // ! game start

    let mut game_num = total_rounds + 1;
    let mut win_num = save_win_num;
    let mut guess_win_try_sum = save_win_try_sum;
    let mut guess_word_record = save_guess_word_record;

    loop {
        // * init alphabet which record state of each letter

        let mut current_game: (String, Vec<String>) = (String::new(), Vec::new()); // (answer, guesses)

        let mut alphabet: HashMap<String, i32> = HashMap::new();
        for i in 97..123
        // from a->z
        {
            alphabet.insert(((i as u8) as char).to_string(), 0);
        }

        // * answer input

        // answer designated by input

        if answer_designated == false && random_mode == false {
            if is_tty {
                print!("Please Enter The Answer:");
                let _ = io::stdout().flush();
            }

            let mut answer_untrimed = String::new();
            io::stdin().read_line(&mut answer_untrimed)?;
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

        // ? guess begin

        current_game.0 = answer.clone().to_ascii_uppercase(); // save current_game answer
        answer = answer.to_ascii_lowercase(); // use lowercase to judge

        let mut guess_time = 1;
        let mut win_or_not = false;
        let mut previous_guess_string = String::from("");

        // (a, b, c) : a=letter, b=index, c=state
        let mut previous_guess_for_difficult: Vec<(char, i32, i32)> = Vec::new();
        let mut yellow_letter_in_previous_guess_max_count: HashMap<char, i32> = HashMap::new();

        while guess_time <= 6 {
            // * guess input

            if is_tty {
                println!("");
                print!("Please Enter Your Guess:");
                let _ = io::stdout().flush();
            }

            let mut guess_untrimed = String::new();
            io::stdin().read_line(&mut guess_untrimed)?;
            let guess = guess_untrimed.trim();

            if acceptable_word_list_use
                .iter()
                .find(|&s| &s.to_ascii_lowercase() == &guess)
                == None
            {
                println!("INVALID");
                continue;
            }

            current_game.1.push(guess.clone().to_ascii_uppercase());

            let guess_vec: Vec<char> = guess.chars().collect(); // process guess to more accessible type

            // * judge validity for difficult mode

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
                    println!("INVALID");
                    continue;
                }
            }

            // * save guess word

            let count = guess_word_record
                .entry(guess.to_string().to_ascii_uppercase())
                .or_insert(0);
            *count += 1;

            let guess_print = guess.clone(); // for colored prin

            // process answer to more accessible type

            let answer_vec: Vec<char> = answer.chars().collect();
            let guess_print_vec: Vec<char> = guess_print.chars().collect();

            let mut match_guess_vec: Vec<bool> = vec![false; 5]; // according to guess match
            let mut match_answer_vec: Vec<bool> = vec![false; 5]; // according to answer match

            let mut right_letter_count = 0;

            // * match begin

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
                if match_guess_vec[guess_index] == true {
                    // previously correctly matched
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

            // * print previous guess if is tty

            if is_tty {
                println!("");
                println!("Your Guess:");
                println!("{}", previous_guess_string);
                previous_guess_string = format!(
                    "{}{}",
                    previous_guess_string.clone(),
                    "\n".to_string().black()
                );
            } else {
                print!(" ");
            }

            // * print alphabet state

            if is_tty {
                println!("");
                println!("Alphabet State:");
                for i in 97..123 {
                    match alphabet.get(&((i as u8) as char).to_string()) {
                        Some(&0) => print!("{}", (i as u8 as char).to_string()),
                        Some(&1) => print!("{}", (i as u8 as char).to_string().red()),
                        Some(&2) => print!("{}", (i as u8 as char).to_string().yellow()),
                        Some(&3) => print!("{}", (i as u8 as char).to_string().green()),
                        _ => unimplemented!(),
                    }
                }
                println!("");
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

            // * win state judge

            if right_letter_count == 5 {
                win_or_not = true;
                break;
            }

            guess_time += 1;
        }

        game.push(current_game);

        // ? guess end

        // * print game reselt

        if is_tty {
            println!("");
            println!("The Game {} Result:", game_num);
        }

        if win_or_not == true {
            if is_tty {
                let print_line = format!("CORRECT! You Guessed {} Times.", guess_time).green();
                println!("{}", print_line);
            } else {
                println!("CORRECT {}", guess_time);
            }
            guess_win_try_sum += guess_time;
            win_num += 1;
        } else {
            if is_tty {
                let print_line =
                    format!("Failed... The Answer is {}.", answer.to_ascii_uppercase()).red();
                println!("{}", print_line);
            } else {
                println!("FAILED {}", answer.to_ascii_uppercase());
            }
        }

        // * print stats if stats_mode on

        if stats_mode == true {
            if is_tty {
                if win_num != 0 {
                    println!(
                        "Win count:{}, Lose count:{}, Average Try count: {:.2}",
                        win_num,
                        game_num - win_num,
                        (guess_win_try_sum as f32) / (win_num as f32)
                    );
                } else {
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
                println!("");
                println!("The Top 5 Words You Guessed Most Frequently:");
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
        }

        // * query for next game

        if answer_designated == true {
            // there's only one round if answer is designated
            break;
        }

        if is_tty {
            println!("");
            println!("Start A New Game or Not? Y for Yes, N for Not");
        }

        total_rounds = game_num;

        let mut line_untrimed = String::new();
        if let Ok(0) = io::stdin().read_line(&mut line_untrimed) {
            break;
        }
        let line = line_untrimed.trim();

        if line == String::from("Y") {
            ()
        } else if line == String::from("N") {
            break;
        }

        day += 1;
        game_num += 1;
    }

    // ! game end

    // * write game_state into save_file

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

    Ok(())
}
