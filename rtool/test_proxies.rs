fn perform_something_on_thing(object: RRef<dyn Thing>) -> u64 {
	let old_domain_id = GET_CALLER_DOMAIN_ID();
	let new_domain_id = GET_CALLEE_DOMAIN_ID();
	RECORD_IP();
	RECORD_SP();
	object.move_to(new_domain_id);
	let ret = perform_something_on_thing(object);
	return ret;
}

impl DomainAInterface for Proxy {
fn read(self&, name: RRef<String>, bytes: usize) -> RRef<[u8]> {
	let old_domain_id = GET_CALLER_DOMAIN_ID();
	let new_domain_id = GET_CALLEE_DOMAIN_ID();
	RECORD_IP();
	RECORD_SP();
	name.move_to(new_domain_id);
	let ret = DOMAIN_A.read(name, bytes);
	ret.move_to(new_domain_id);
	return ret;
}

fn write(self&, name: RRef<String>, bytes: RRef<[u8]>) -> usize {
	let old_domain_id = GET_CALLER_DOMAIN_ID();
	let new_domain_id = GET_CALLEE_DOMAIN_ID();
	RECORD_IP();
	RECORD_SP();
	name.move_to(new_domain_id);
	bytes.move_to(new_domain_id);
	let ret = DOMAIN_A.write(name, bytes);
	return ret;
}

fn test3(self&, arg1: RRef<Type1>, arg2: RRef<Type1>) {
	let old_domain_id = GET_CALLER_DOMAIN_ID();
	let new_domain_id = GET_CALLEE_DOMAIN_ID();
	RECORD_IP();
	RECORD_SP();
	arg1.move_to(new_domain_id);
	arg2.move_to(new_domain_id);
	let ret = DOMAIN_A.test3(arg1, arg2);
	return ret;
}

}
