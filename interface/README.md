# RPC Types
1. Types that implements `core::marker::Copy` trait beside function pointers

# Interface syntax.
## RPC
See [RPC Types](#rpc_types) for valid RPC Types.
```
#[interface]
pub trait YourInterface {
    fn method_name(&self, [arg: RPCType],*) -> RpcResult<RPCType>;
    ... more methods ...
}
```


# How to add a new domain.
1. Add a domain create interface inside of the `domain_create` module.
    ```
        #[domain_create(path = "my_domain_name")]
        pub trait CreateYourDomain: Send + Sync {
            fn create_domain_your_domain(&self) -> (Box<dyn Domain>, Box<dyn YourDomain>);
        }
    ```
1. Add it inside of `interface::proxy::Proxy` as a method of the proxy.
    ```
        fn as_domain_create_CreateDomC1(&self) -> Arc<dyn crate::domain_create::CreateDomC1>;
    ```
1. Update the proxy object instantiation in _domains/usr/proxy/src/main.rs:trusted\_entry_ so the 
    proxy will contain an instance of `CreateYourDomain`.
1. Update the proxy domain entry point in both `interface::domain_create::CreateProxy` and 
    _domains/usr/proxy/src/main.rs:trusted\_entry_ to allow the instance of `CreateYourDomain` be 
    passed down from domain `redleaf_init` to domain `proxy`.
1. Update the entry point and the instantiation of the proxy domain of `redleaf_init` in 
    _domains/sys/init/src/main.rs:trusted\_entry_. Make sure the signature of the entry point is
    the same as the one _kernel/src/generated\_domain\_create.rs:create\_domain\_init_

