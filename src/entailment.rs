use egg::{*, rewrite as rw};

define_language! {
    pub enum ImpExpr {
        "+" = Add([Id; 2]),
        "-" = Sub([Id; 2]),
        "*" = Mul([Id; 2]),
        "^" = Pow([Id; 2]),
        "=" = Eq([Id; 2]),
        "#" = Ne([Id; 2]),
        "<=" = Le([Id; 2]),
        "<" = Lt([Id; 2]),
        ">=" = Ge([Id; 2]),
        ">" = Gt([Id; 2]),

        "&&" = And([Id; 2]),
        "||" = Or([Id; 2]),
        "!" = Not(Id),

        Num(i64),
        Bool(bool),
        Var(Symbol),
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Value {
    Number(i64),
    Boolean(bool),
}

impl Value {
    fn num(&self) -> i64 {
        match self {
            Value::Number(n) => *n,
            Value::Boolean(_) => panic!("called num() on bool-Value"),
        }
    }

    fn bool(&self) -> bool {
        match self {
            Value::Number(_) => panic!("called bool() on number-Value"),
            Value::Boolean(b) => *b,
        }
    }
}

// TODO: do constant propagation somehow over equality: "x = y" and y is known to be constant, also make x constant
#[derive(Default)]
struct ConstantFolding;
impl Analysis<ImpExpr> for ConstantFolding {
    type Data = Option<Value>;

    fn make(egraph: &EGraph<ImpExpr, Self>, enode: &ImpExpr) -> Self::Data {
        let x = |i: &Id| egraph[*i].data;
        match enode {
            ImpExpr::Num(n) => Some(Value::Number(*n)),
            ImpExpr::Bool(b) => Some(Value::Boolean(*b)),

            ImpExpr::Add([a, b]) => Some(Value::Number(x(a)?.num() + x(b)?.num())),
            ImpExpr::Sub([a, b]) => Some(Value::Number(x(a)?.num() - x(b)?.num())),
            ImpExpr::Mul([a, b]) => Some(Value::Number(x(a)?.num() * x(b)?.num())),
            ImpExpr::Pow([a, b]) => Some(Value::Number(x(a)?.num().pow(x(b)?.num() as u32))),

            ImpExpr::Eq([a, b]) => Some(Value::Boolean(x(a)?.num() == x(b)?.num())),
            ImpExpr::Ne([a, b]) => Some(Value::Boolean(x(a)?.num() != x(b)?.num())),
            ImpExpr::Le([a, b]) => Some(Value::Boolean(x(a)?.num() <= x(b)?.num())),
            ImpExpr::Lt([a, b]) => Some(Value::Boolean(x(a)?.num() < x(b)?.num())),
            ImpExpr::Ge([a, b]) => Some(Value::Boolean(x(a)?.num() >= x(b)?.num())),
            ImpExpr::Gt([a, b]) => Some(Value::Boolean(x(a)?.num() > x(b)?.num())),

            ImpExpr::And([a, b]) => Some(Value::Boolean(x(a)?.bool() && x(b)?.bool())),
            ImpExpr::Or([a, b]) => Some(Value::Boolean(x(a)?.bool() || x(b)?.bool())),
            ImpExpr::Not(a) => Some(Value::Boolean(!x(a)?.bool())),
            _ => None,
        }
    }

    fn merge(&self, to: &mut Self::Data, from: Self::Data) -> bool {
        egg::merge_if_different(to, to.or(from))
    }

    /*
    a = 5;
    x = 10;
    return x;

    <=>
    b = 5;
    y = 10;
    return y;

    */

    fn modify(egraph: &mut EGraph<ImpExpr, Self>, id: Id) {
        match egraph[id].data {
            Some(Value::Boolean(b)) => {
                let added = egraph.add(ImpExpr::Bool(b));
                egraph.union(id, added);
            }
            Some(Value::Number(n)) => {
                let added = egraph.add(ImpExpr::Num(n));
                egraph.union(id, added);
            }
            _ => {}
        }
    }
}

pub fn example() {


    let my_expr: RecExpr<ImpExpr> = "(^ 2 (+ z 1))".parse().unwrap();
    let my_expr: RecExpr<ImpExpr> = "(* (^ 2 z) (^ 2 1))".parse().unwrap();

    let eq_expr1: RecExpr<ImpExpr> = "(= (* y 2) (^ 2 (+ z 1)))".parse().unwrap();
    let eq_expr2: RecExpr<ImpExpr> = "(= y (^ 2 z))".parse().unwrap();

    // let runner = Runner::default().with_expr(&eq_expr1).with_expr(&eq_expr2).run(&rules());
    // let mut extractor = Extractor::new(&runner.egraph, AstSize);
    //
    // println!("Ids of roots: {:?}", runner.roots.iter().map(|id| runner.egraph.find(*id)).collect::<Vec<_>>());
    //
    // let (best_cost, best_expr) = extractor.find_best(runner.roots[0]);
    //
    //
    // println!("Found bests: {} {}", extractor.find_best(runner.roots[0]).1, extractor.find_best(runner.roots[1]).1)

    let bests = get_bests(vec![&eq_expr1, &eq_expr2]);
    println!("Found bests: {}", bests.into_iter().map(|recexpr| recexpr.to_string()).collect::<Vec<_>>().join(", "));


    let dangerous_rw: RecExpr<ImpExpr> = "(= (* 0 2) (* 0 5))".parse().unwrap();

    let bests = get_bests(vec![&dangerous_rw]);
    println!("Found bests: {}", bests.into_iter().map(|recexpr| recexpr.to_string()).collect::<Vec<_>>().join(", "));

    // let from_ax: RecExpr<ImpExpr> = "(&& (= y (^ 2 z)) (&& (= x X) (= z x)))".parse().unwrap();
    // let mut runner = Runner::default().with_expr(&from_ax).run(&rules());
    //
    // let pat: Pattern<ImpExpr> = "(&& (= ?x ?y) ?z)".parse().unwrap();
    // let sm = pat.search(&runner.egraph);
    // for sm in sm {
    //     println!("\nfound searchmatch: {:?}", &sm);
    //     let zv: Var = "?z".parse().unwrap();
    //     let xv: Var = "?x".parse().unwrap();
    //     let yv: Var = "?y".parse().unwrap();
    //     for subst in sm.substs {
    //         let z_id = subst[zv];
    //         let x_id = subst[xv];
    //         let y_id = subst[yv];
    //
    //
    //         let mut extractor = Extractor::new(&runner.egraph, AstSize);
    //         let (_, z_best) = extractor.find_best(z_id);
    //         let (_, x_best) = extractor.find_best(x_id);
    //         let (_, y_best) = extractor.find_best(y_id);
    //         println!("Best z: {}", z_best.to_string());
    //         println!("Best x: {}", x_best.to_string());
    //         println!("Best y: {}", y_best.to_string());
    //         let x_pat: Pattern<ImpExpr> = x_best.to_string().parse().unwrap();
    //         let y_pat: Pattern<ImpExpr> = y_best.to_string().parse().unwrap();
    //         //apply substs from x to y
    //         println!("x_pat: {:?}", x_pat.search_eclass(&runner.egraph, z_id));
    //         println!("y_pat: {:?}", y_pat.search_eclass(&runner.egraph, z_id));
    //
    //         // println!("searching for ?x: {:?}", pat.search_eclass(&runner.egraph, z_id));
    //
    //         // have ?x, want to substitute with ?y
    //         let _ignored = x_id;
    //
    //         let other_x_pattern: Pattern<ImpExpr> = "?x".parse().unwrap();
    //         let mut sub = Subst::with_capacity(1);
    //         sub.insert(xv, y_id);
    //         let res = other_x_pattern.apply_one(&mut runner.egraph, _ignored, &sub);
    //         runner.egraph.union(res, )
    //
    //         y_pat.apply_one(&mut runner.egraph, _ignored, )
    //
    //     }
    //
    //
    // }


    // let intro_or_exp1: RecExpr<ImpExpr> = "a".parse().unwrap();
    // let intro_or_exp2: RecExpr<ImpExpr> = "(|| a b)".parse().unwrap();
    //
    // let mut runner = Runner::default().with_expr(&intro_or_exp1).with_expr(&intro_or_exp2).run(&rules());
    //
    // println!("intro_or equivs: {:?}", runner.egraph.equivs(&intro_or_exp1, &intro_or_exp2));
    //
    // let bests = get_bests(vec![&intro_or_exp1, &intro_or_exp2]);
    // println!("Found intro_or bests: {}", bests.into_iter().map(|recexpr| recexpr.to_string()).collect::<Vec<_>>().join(", "));


}

pub fn get_bests(exprs: Vec<&RecExpr<ImpExpr>>) -> Vec<RecExpr<ImpExpr>> {
    let runner = exprs.into_iter()
        .fold(Runner::default(), |runner, expr| runner.with_expr(expr))
        .run(&rules());
    let mut extractor = Extractor::new(&runner.egraph, AstSize);

    runner.roots.iter().map(|id| extractor.find_best(*id).1).collect()
}

fn rules() -> Vec<Rewrite<ImpExpr, ConstantFolding>> { vec![
    rw!("commute-add"; "(+ ?x ?y)" => "(+ ?y ?x)"),
    rw!("commute-mul"; "(* ?x ?y)" => "(* ?y ?x)"),
    rw!("commute-eq"; "(= ?x ?y)" => "(= ?y ?x)"),

    rw!("commute-and"; "(&& ?x ?y)" => "(&& ?y ?x)"),
    rw!("commute-or"; "(|| ?x ?y)" => "(|| ?y ?x)"),

    // rw!("intro-or"; "?x" => "(|| ?x ?y)"), // Not possible currently

    rw!("ass-and"; "(&& (&& ?x ?y) ?z)" => "(&& ?x (&& ?y ?z))"),
    rw!("ass-or"; "(|| (|| ?x ?y) ?z)" => "(|| ?x (|| ?y ?z))"),

    rw!("double-not"; "(! (! ?x))" => "?x"),

    // rw!("trans-eq"; "(&& (= ?x ?y) (= ?y ?z))" => "(= ?x ?z)"), //TODO: dangerous because we "lose" information (in the best expression)

    rw!("neg-lt"; "(! (< ?x ?y))" => "(>= ?x ?y)"),
    rw!("neg-le"; "(! (<= ?x ?y))" => "(> ?x ?y)"),
    rw!("neg-gt"; "(! (> ?x ?y))" => "(<= ?x ?y)"),
    rw!("neg-ge"; "(! (>= ?x ?y))" => "(< ?x ?y)"),

    rw!("le-and-ge"; "(&& (<= ?x ?y) (>= ?x ?y))" => "(= ?x ?y)"),
    rw!("lt-or-gt"; "(|| (< ?x ?y) (> ?x ?y))" => "(# ?x ?y)"),

    rw!("add-0"; "(+ ?x 0)" => "?x"),
    rw!("mul-0"; "(* ?x 0)" => "0"),
    rw!("mul-1"; "(* ?x 1)" => "?x"),
    rw!("sub-self"; "(- ?x ?x)" => "0"),
    rw!("pow-split"; "(^ ?x (+ ?y ?z))" => "(* (^ ?x ?y) (^ ?x ?z))"),
    rw!("pow-unsplit"; "(* (^ ?x ?y) (^ ?x ?z))" => "(^ ?x (+ ?y ?z))"),
    // rw!("pow-1"; "(^ ?x 1)" => "?x"),

    rw!("eq-elim-add"; "(= (+ ?x ?y) (+ ?x ?z))" => "(= ?y ?z)"),
    rw!("eq-elim-sub"; "(= (- ?y ?x) (- ?z ?x))" => "(= ?y ?z)"),
    rw!("eq-elim-mul"; "(= (* ?x ?y) (* ?x ?z))" => "(= ?y ?z)" if is_not_zero("?x")),
    rw!("eq-sub-1"; "(= (+ ?x ?y) ?z)" => "(= ?x (- ?z ?y))"),
    rw!("eq-sub-2"; "(= (- ?z ?y) ?x)" => "(= ?z (+ ?x ?y))"),



    // rw!("eq-elim.general"; "(= (?o ?x ?y) (?o ?x ?z))" => "(= ?y ?z)"), // Not possible currently

] }

fn is_not_zero(var: &'static str) -> impl Fn(&mut EGraph<ImpExpr, ConstantFolding>, Id, &Subst) -> bool {
    let var = var.parse().unwrap();
    let zero = ImpExpr::Num(0);
    move |egraph, _, subst| !egraph[subst[var]].nodes.contains(&zero)
}
