.PHONY: checkstack
checkstack: checkstackinfo
	$(eval greater := $(shell if [ $(max_stack) -gt $(half_ukern_stack) ]; then echo fail; fi))
	$(if $(greater), $(error "This domain uses stack of $(max_stack) bytes which is larger than half of the stack allocated by the microkernel ($(half_ukern_stack))))

.PHONY: checkstackinfo
checkstackinfo:
	$(eval max_stack := $(shell stack-sizes $(bin) | sort -k2 -nr | awk 'NR==1{print $$2}'))
	$(eval half_ukern_stack := $(shell grep "^pub const STACK_SIZE_IN_PAGES" $(root)/src/thread.rs | grep -o '[[:digit:]]*' | awk '{print $$1*4096/2}'))
	$(eval max_stacks := $(shell stack-sizes $(bin) | sort -k2 -nr | head -n 1))
	@echo "Max allocated stack used by this domain is $(max_stack) bytes, which is less than half of kernel stack ($(half_ukern_stack) bytes)"
	@echo "The largest stack is allocated by this function:" 
	@echo "$(max_stacks)"
	@echo "You can use this command for detailed info on stack usage in this domain"
	@echo "    $ stack-sizes $(bin) | sort -k2 -nr"

