
MAINS ?= main

BIBTEX ?= biber # I like biber as a default
LATEX ?= pdflatex
LATEX_FLAGS = -shell-escape -halt-on-error -output-directory _build/

TEX_FILES = $(shell find . -name '*.tex' -or -name '*.sty' -or -name '*.cls')
BIB_FILES = $(shell find . -name '*.bib')
BST_FILES = $(shell find . -name '*.bst')

default: $(MAINS:%=_build/%.pdf)

.PHONY: clean
clean:
	rm -r _build/

_build/:
	mkdir -p _build/

_build/%.aux: $(TEX_FILES) $(EXTRA_FILES) | _build/
	$(LATEX) $(LATEX_FLAGS) $*.tex

_build/%.bbl: $(BIB_FILES) | _build/%.aux
	$(BIBTEX) _build/$*
	$(LATEX) $(LATEX_FLAGS) $*.tex

_build/%.pdf: _build/%.aux $(if $(BIB_FILES), _build/%.bbl)
	$(LATEX) $(LATEX_FLAGS) $*.tex
