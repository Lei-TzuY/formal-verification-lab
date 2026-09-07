use formal_verification_lab::{
    check_multi_response, check_multi_response_with_fairness_profile,
    check_multi_response_with_fairness_profile_and_limits,
    check_multi_response_with_product_limits, execute_multi_response, parse_verification_job,
    AnalysisLimits, FairnessProfile, Invariant, MultiResponseExecutionConfig,
    MultiResponseExecutionResult, MultiResponseProperty, ResponseClause, StateVariable, Transition,
    TransitionSystem,
};

fn model() -> TransitionSystem<usize> {
    TransitionSystem::new(
        "execution-model",
        vec![StateVariable::new("node", "node")],
        vec![0usize],
        |state| match *state {
            0 => Ok(vec![Transition::new("request", 1usize)]),
            1 => Ok(vec![
                Transition::new("wait", 1usize),
                Transition::new("grant", 0usize),
            ]),
            _ => Ok(Vec::new()),
        },
        vec![Invariant::new("domain", |state: &usize| *state <= 1)],
    )
    .unwrap()
}

fn property() -> MultiResponseProperty {
    MultiResponseProperty::new(
        "request-response",
        vec![ResponseClause::new(
            "request",
            |action| action == "request",
            |action| action == "grant",
        )
        .unwrap()],
    )
    .unwrap()
}

#[test]
fn no_options_dispatch_is_exactly_unbounded_backend() {
    let model = model();
    let property = property();
    let job = parse_verification_job("model \"m\"\nproperty \"p\"\n").unwrap();
    let config = MultiResponseExecutionConfig::from_job(&job).unwrap();

    let actual = execute_multi_response(&model, &property, &config).unwrap();
    let expected = check_multi_response(&model, &property).unwrap();

    assert_eq!(actual, MultiResponseExecutionResult::Unbounded(expected));
}

#[test]
fn weak_fair_job_dispatches_through_canonical_profile_backend() {
    let model = model();
    let property = property();
    let job = parse_verification_job(
        "model \"m\"\nproperty \"p\"\nweak-fair-action \"grant\"\n",
    )
    .unwrap();
    let config = MultiResponseExecutionConfig::from_job(&job).unwrap();
    let fairness = FairnessProfile::new(["grant"], std::iter::empty::<&str>()).unwrap();

    let actual = execute_multi_response(&model, &property, &config).unwrap();
    let expected = check_multi_response_with_fairness_profile(&model, &property, &fairness).unwrap();

    assert_eq!(actual, MultiResponseExecutionResult::Unbounded(expected));
}

#[test]
fn product_budget_dispatch_is_exactly_product_bounded_backend() {
    let model = model();
    let property = property();
    let job = parse_verification_job(
        "model \"m\"\nproperty \"p\"\nmax-product-transitions 1\n",
    )
    .unwrap();
    let config = MultiResponseExecutionConfig::from_job(&job).unwrap();

    let actual = execute_multi_response(&model, &property, &config).unwrap();
    let expected = check_multi_response_with_product_limits(
        &model,
        &property,
        job.product_limits(),
    )
    .unwrap();

    assert_eq!(
        actual,
        MultiResponseExecutionResult::ProductBounded(expected)
    );
}

#[test]
fn staged_fair_job_dispatch_is_exactly_staged_profile_backend() {
    let model = model();
    let property = property();
    let job = parse_verification_job(
        "model \"m\"\nproperty \"p\"\nweak-fair-action \"grant\"\nmax-model-transitions 2\nmax-product-transitions 3\n",
    )
    .unwrap();
    let config = MultiResponseExecutionConfig::from_job(&job).unwrap();
    let fairness = FairnessProfile::new(["grant"], std::iter::empty::<&str>()).unwrap();
    let limits = AnalysisLimits::new(job.model_limits(), job.product_limits());

    let actual = execute_multi_response(&model, &property, &config).unwrap();
    let expected = check_multi_response_with_fairness_profile_and_limits(
        &model,
        &property,
        &fairness,
        limits,
    )
    .unwrap();

    assert_eq!(actual, MultiResponseExecutionResult::Staged(expected));
}
