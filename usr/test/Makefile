.PHONY: all
all:
	make -C dom_a
	make -C dom_b
	make -C dom_c
	make -C dom_d
	make -C shadow
	make -C benchnet_inside
	make -C benchnvme
	make -C benchhash
	make -C testtpm

.PHONY: clean
clean:
	make -C dom_a clean
	make -C dom_b clean
	make -C dom_c clean
	make -C dom_d clean
	make -C shadow clean
	make -C benchnet_inside clean
	make -C benchnvme clean
	make -C benchhash clean
	make -C testtpm clean
