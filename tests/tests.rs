use rocket_dependency_injection::derive::{resolve_constructor, Resolve};
use rocket_dependency_injection::{RocketExtension, ServiceProvider};

#[tokio::test]
async fn test_add_with() {
    #[derive(Clone)]
    struct A;
    #[derive(Clone)]
    struct B(A);

    let rocket = rocket::build()
        .add_with(|provider| B(provider.unwrap()))
        .add_with(|_| A)
        .ignite()
        .await
        .unwrap();

    assert!(rocket.state::<B>().is_some());
}

#[tokio::test]
async fn test_add() {
    #[derive(Clone)]
    struct A;
    #[derive(Clone)]
    struct B(A);

    impl rocket_dependency_injection::Resolve for A {
        fn resolve(_ : &ServiceProvider) -> Self {
            Self
        }
    }

    impl rocket_dependency_injection::Resolve for B {
        fn resolve(service_provider : &ServiceProvider) -> Self {
            Self(service_provider.unwrap())
        }
    }

    let rocket = rocket::build()
        .add::<B>()
        .add::<A>()
        .ignite()
        .await
        .unwrap();

    assert!(rocket.state::<B>().is_some());
}

#[tokio::test]
async fn test_add_derive() {
    #[derive(Clone, Resolve)]
    struct A;
    #[derive(Clone, Resolve)]
    struct B(A);

    impl A {
        #[resolve_constructor]
        fn new() -> Self {
            Self
        }
    }

    impl B {
        #[resolve_constructor]
        fn new(a: A) -> Self {
            Self(a)
        }
    }

    let rocket = rocket::build()
        .add::<B>()
        .add::<A>()
        .ignite()
        .await
        .unwrap();

    assert!(rocket.state::<B>().is_some());
}