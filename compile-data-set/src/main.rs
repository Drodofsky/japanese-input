use std::{fs::File, io::Read};

use hands::HandParser;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
fn main() {
    let mut res: Vec<_> = (1..164)
        .into_par_iter()
        .map(|i| {
            let mut raw = Vec::new();
            let mut file =
                File::open(format!("../data/HANDS-nakayosi_t-98-09/NKY{i:04}.TXT")).unwrap();
            file.read_to_end(&mut raw).unwrap();
            let parser = HandParser::new().parse(&raw).unwrap();
            let stroke_data = parser.get_stroke_data();
            let out_of_frame: u32 = stroke_data
                .iter()
                .map(|c| {
                    c.1.iter()
                        .map(|s| {
                            s.iter()
                                .map(|p| {
                                    if p.0 < 0.0 || p.0 > 1.0 || p.1 < 0.0 || p.1 > 1.0 {
                                        1
                                    } else {
                                        0
                                    }
                                })
                                .sum::<u32>()
                        })
                        .sum::<u32>()
                })
                .sum();
            let sex = match parser.get_sex() {
                "m" => 'm',
                "w" => 'w',
                x => panic!("unexpected sex: {x}"),
            };
            println!("writer({}) with {} points out of frame", sex, out_of_frame);
            (i, sex, out_of_frame, stroke_data.to_owned())
        })
        .collect();
    println!("for testing: ");
    res.sort_by_key(|(_, sex, _out_of_frame, _)| *sex);
    let split_index = res
        .iter()
        .enumerate()
        .rfind(|(_i, c)| c.1 == 'm')
        .unwrap()
        .0;
    let men = &mut res[0..=split_index];
    men.sort_by_key(|(_, _sex, out_of_frame, _)| *out_of_frame);
    let men_low_index = ((men.len() as f64) * 0.25) as usize;
    let men_mid_index = ((men.len() as f64) * 0.50) as usize;
    let men_high_index = ((men.len() as f64) * 0.75) as usize;
    println!("men: ");
    println!(
        "selected NKY{:04}.TXT, with {} points out of frame",
        res[men_low_index].0, res[men_low_index].2
    );
    println!(
        "selected NKY{:04}.TXT, with {} points out of frame",
        res[men_mid_index].0, res[men_mid_index].2
    );
    println!(
        "selected NKY{:04}.TXT, with {} points out of frame",
        res[men_high_index].0, res[men_high_index].2
    );

    let women = &mut res[split_index + 1..];
    women.sort_by_key(|(_, _sex, out_of_frame, _)| *out_of_frame);
    let women_low_index = ((women.len() as f64) * 0.25) as usize + split_index + 1;
    let women_mid_index = ((women.len() as f64) * 0.50) as usize + split_index + 1;
    let women_high_index = ((women.len() as f64) * 0.75) as usize + split_index + 1;
    println!("women:");
    println!(
        "selected NKY{:04}.TXT, with {} points out of frame",
        res[women_low_index].0, res[women_low_index].2
    );
    println!(
        "selected NKY{:04}.TXT, with {} points out of frame",
        res[women_mid_index].0, res[women_mid_index].2
    );
    println!(
        "selected NKY{:04}.TXT, with {} points out of frame",
        res[women_high_index].0, res[women_high_index].2
    );

    let mut test = Vec::new();
    test.append(&mut res.remove(women_high_index).3);
    test.append(&mut res.remove(women_mid_index).3);
    test.append(&mut res.remove(women_low_index).3);
    test.append(&mut res.remove(men_high_index).3);
    test.append(&mut res.remove(men_mid_index).3);
    test.append(&mut res.remove(men_low_index).3);
    let mut optimize = Vec::new();
    for (_, _, _, mut stroke_data) in res {
        optimize.append(&mut stroke_data);
    }

    println!("for testing: {} characters", test.len());
    println!("for optimizing: {} characters", optimize.len());
    let test_data = postcard::to_allocvec(&test).expect("Failed to serialize");
    std::fs::write("../data/generated/hands_test.bin", &test_data).expect("Failed to write");
    let optimize_data = postcard::to_allocvec(&optimize).expect("Failed to serialize");
    std::fs::write("../data/generated/hands_optimize.bin", &optimize_data)
        .expect("Failed to write");
    assert_eq!(test.len() + optimize.len(), 1695689);
    println!(
        "Wrote {} entries ({} bytes)",
        test.len() + optimize.len(),
        test_data.len() + optimize_data.len()
    );
}
