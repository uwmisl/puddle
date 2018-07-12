#!/usr/bin/make -f

# path to the rust project file
cargo_path=${HOME}/src/puddle/src/core/
vision_cpp=${cargo_path}/src/vision/droplet_detect.cpp

## paths to set
# where the output files are made
out = /tmp/pi-pics-${USER}
# where original images are
original_path = ./data
# where the labeled gimp files are, if we need to make the labeled images
xcf_path = ./xcf
# where labeled images are, or will be made
labeled_path = ./labeled

# convenience variables
original_files = $(wildcard ${original_path}/*.jpg)
basenames = $(original_files:${original_path}/%.jpg=%)
diff_files = $(basenames:%=${out}/diff/%.json)


all: ${out}/summary.json

# shortcuts are phony because they don't create files
.PHONY: clean guessed labeled diff summary

clean:
	-rm -r ${out}/guessed/
	-rm -r ${out}/diff/
	-rm ${out}/summary.json

# only remove the labeled images if we really need to
veryclean: clean
	-rm -r ${labeled_path}

labeled: $(basenames:%=${labeled_path}/%.png)
${labeled_path}/%.png: ${xcf_path}/%.xcf
	@mkdir -p ${labeled_path}
	gimp -i -b " \
    (let* ((img (car (gimp-file-load RUN-NONINTERACTIVE \"$^\" \"$^\"))) \
           (layer (car (gimp-image-merge-visible-layers img CLIP-TO-IMAGE)))) \
        (gimp-file-save RUN-NONINTERACTIVE img layer \"$@\" \"$@\")) \
    (gimp-quit FALSE)" > /dev/null 2>&1

guessed: $(basenames:%=${out}/guessed/%.png)
${out}/guessed/%.png: ${original_path}/%.jpg ${vision_cpp}
	@mkdir -p ${out}/guessed
	cargo run --quiet --manifest-path ${cargo_path}/Cargo.toml --features vision --bin vision-test -- file $< $@

diff: ${diff_files}
${out}/diff/%.json: ${out}/guessed/%.png ${labeled_path}/%.png img_diff.py
	@mkdir -p ${out}/diff
	python3 img_diff.py --guessed $(word 1,$^) --labeled $(word 2,$^) --output $@

summary: ${out}/summary.json
${out}/summary.json: ${diff_files} summary.py
	python3 summary.py --verbose --output $@ $(filter-out summary.py, $^)
