use std::{
    any::type_name,
    sync::{Arc, RwLock},
};

#[cfg(feature = "derive")]
pub use rocket_dependency_injection_derive as derive;

use rocket::{
    fairing::{Fairing, Info, Kind},
    Build, Rocket,
};

pub struct ServiceProvider {
    inner: Rocket<Build>,
}

impl From<Rocket<Build>> for ServiceProvider {
    fn from(value: Rocket<Build>) -> Self {
        ServiceProvider { inner: value }
    }
}

impl ServiceProvider {
    pub fn unwrap<T>(&self) -> T
    where
        T: Clone + Send + Sync + 'static,
    {
        let type_name = type_name::<T>();
        match self.inner.state::<T>() {
            None => self
                .inner
                .state::<Arc<ServiceResolver<_>>>()
                .map(|resolver| resolver.resolve(&self)),
            other => other.map(|item| item.clone()),
        }
        .expect(format!("Failed to resolve service of type {}", type_name).as_str())
    }
}

struct ServiceResolver<TInjectedItem> {
    injection_function: Box<dyn Fn(&ServiceProvider) -> TInjectedItem + Send + Sync + 'static>,
    item: RwLock<Option<TInjectedItem>>,
}

impl<TInjectedItem> ServiceResolver<TInjectedItem>
where
    TInjectedItem: Clone + Send + Sync + 'static,
{
    pub fn new<
        TResolutionFunction: Fn(&ServiceProvider) -> TInjectedItem + Send + Sync + 'static,
    >(
        injection_function: TResolutionFunction,
    ) -> Self {
        Self {
            injection_function: Box::new(injection_function),
            item: RwLock::new(None),
        }
    }

    pub fn resolve(&self, service_provider: &ServiceProvider) -> TInjectedItem {
        {
            if let Some(ref item) = *self.item.read().unwrap() {
                return item.clone();
            }
        }

        let mut guard = self.item.write().unwrap();

        let item = (self.injection_function)(service_provider);
        *guard = Some(item.clone());

        item
    }
}

#[async_trait::async_trait]
impl<TResolvedItem> Fairing for ServiceResolver<TResolvedItem>
where
    TResolvedItem: Clone + Sync + Send + 'static,
{
    fn info(&self) -> Info {
        Info {
            name: Box::leak(format!("{}_resolver", type_name::<TResolvedItem>()).into_boxed_str()),
            kind: Kind::Ignite,
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> Result<Rocket<Build>, Rocket<Build>> {
        let service_provider: ServiceProvider = rocket.into();
        let item = (self.injection_function)(&service_provider);
        Ok(service_provider.inner.manage(item))
    }
}

pub trait Resolve {
    fn resolve(service_provider: &ServiceProvider) -> Self;
}

pub trait RocketExtension {
    fn add_with<
        TInjectedItem: Clone + Sync + Send + 'static,
        TInjectionFunction: Fn(&ServiceProvider) -> TInjectedItem + Send + Sync + 'static,
    >(
        self,
        injection_function: TInjectionFunction,
    ) -> Self;

    fn add<TResolve: Resolve + Send + Sync + Clone + 'static>(self) -> Self;
}

impl RocketExtension for Rocket<Build> {
    fn add_with<
        TInjectedItem: Clone + Sync + Send + 'static,
        TInjectionFunction: Fn(&ServiceProvider) -> TInjectedItem + Send + Sync + 'static,
    >(
        self,
        injection_function: TInjectionFunction,
    ) -> Self {
        let service_resolver = Arc::new(ServiceResolver::new(injection_function));

        self.attach(service_resolver.clone())
            .manage(service_resolver)
    }

    fn add<TResolve: Resolve + Send + Sync + Clone + 'static>(self) -> Self {
        self.add_with(TResolve::resolve)
    }
}
