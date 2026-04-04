.PHONY: srpm clean

NAME := $(shell sed -n 's/^Name:[[:space:]]*//p' "$(spec)" | head -n1)
VERSION := $(shell sed -n 's/^Version:[[:space:]]*//p' "$(spec)" | head -n1)
TARBALL := $(outdir)/$(NAME)-$(VERSION).tar.gz

srpm:
	@test -n "$(outdir)" || { echo "error: outdir is required"; exit 1; }
	@test -n "$(spec)" || { echo "error: spec is required"; exit 1; }
	mkdir -p "$(outdir)"
	git archive --format=tar.gz \
		--prefix=$(NAME)-$(VERSION)/ \
		-o "$(TARBALL)" HEAD
	rpmbuild -bs "$(spec)" \
		--define "_sourcedir $(outdir)" \
		--define "_srcrpmdir $(outdir)"

clean:
	@test -n "$(outdir)" || { echo "error: outdir is required"; exit 1; }
	rm -f -- "$(TARBALL)" "$(outdir)"/*.src.rpm
