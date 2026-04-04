SPEC := modpdb.spec
NAME := $(shell sed -n 's/^Name:[[:space:]]*//p' $(SPEC) | head -n1)
VERSION := $(shell sed -n 's/^Version:[[:space:]]*//p' $(SPEC) | head -n1)
TARBALL := $(NAME)-$(VERSION).tar.gz

.PHONY: srpm clean

srpm: $(TARBALL)
	rpmbuild -bs $(SPEC) \
		--define "_sourcedir $(CURDIR)" \
		--define "_srcrpmdir $(CURDIR)"

$(TARBALL):
	git archive --format=tar.gz \
		--prefix=$(NAME)-$(VERSION)/ \
		-o $@ HEAD

clean:
	rm -f -- $(TARBALL) *.src.rpm
