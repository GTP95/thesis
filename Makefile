pdf:
	pdflatex -output-directory latex_out thesis_plan
	biber --input-directory latex_out --output-directory latex_out thesis_plan
	pdflatex -output-directory latex_out thesis_plan
	pdflatex -output-directory latex_out thesis_plan

clean:
	rm -r latex_out/*
