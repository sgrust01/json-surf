use json_surf::prelude::*;

fn main() {
    let word = FuzzyWord::default();
    let correct = "saurav";
    let result = word.lookup(correct);
    println!("Correct: {} Suggested: {:#?}", correct, result.unwrap());
    let correct = "sauarv";
    let result = word.lookup(correct);
    println!("Correct: {} Suggested: {:#?}", correct, result.unwrap());
    let correct = "sauar";
    let result = word.lookup(correct);
    println!("Correct: {} Suggested: {:#?}", correct, result.unwrap());
    let correct = "saurab";
    let result = word.lookup(correct);
    println!("Correct: {} Suggested: {:#?}", correct, result.unwrap());
}