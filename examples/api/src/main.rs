use rug::*;

fn main() {
    let mut pb = path::PathBuilder::new();
    pb.move_to([1.0, 1.0].into());
    pb.line_to([9.0, 1.0].into());
    pb.line_to([5.0, 4.0].into());
    pb.close();

    let path_buf = pb.build();
    let path = path_buf.path();

    println!("{:?}", path.verbs());
    println!("{:?}", path.points());
    println!("{:?}", path.aabb());
}

