trait X<T>

trait Y {
    type TraitT;
}


struct MyStruct;

// X can be implemented for each different type parameter
impl X<i32> for MyStruct {}
impl X<bool> for MyStruct {}
impl X<f64> for MyStruct {}

// in your function or whatever, you can require that it implements something
fn handle<T>(x: impl X<T>)
    where T: std::fmt::Debug,
{}

// Y can only be implemented once on each type. The type can only be declared once
impl Y for MyStruct {
    type TraitT = bool;
}

// You can restrict it too
fn handle2<T>(x: impl Y<TraitT = T>)
    where T: std::fmt::Debug,
{}