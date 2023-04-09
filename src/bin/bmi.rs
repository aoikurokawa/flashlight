fn main() {
   let height_cm = input("Enter height(cm): ");
   let weight_kg = input("Enter weight(kg): ");

   let height = height_cm / 100.0;
   let bmi = weight_kg / height.powf(2.0);
   println!("BMI={:.1}", bmi);

   if bmi < 18.5 {
      println!("low weight");
   } else if bmi < 25.0 {
      println!("normal weight");
   } else if bmi <  30.0 {
      println!("1 degree");
   } else if bmi < 35.0 {
      println!("2 degree");
   } else if bmi < 40.0 {
      println!("3 degree");
   } else {
      println!("4 degree");
   }
}

fn input(prompt: &str) -> f64 {
   println!("{}", prompt);

   let mut s = String::new();
   std::io::stdin().read_line(&mut s).expect("error");
   return s.trim().parse().expect("error");
}


