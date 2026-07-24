
all: xml json

xml:
	tidy -iqm -wrap 0 -xml splits/*.lss

json:
	for i in splits/*.ls1l; do \
	    jq --indent 4 . "$$i" > "$${i}.tmp" && mv "$${i}.tmp" "$$i" ; \
	done

clean:
	rm splits/*.tmp
