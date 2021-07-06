.PHONY: js/proto

js/proto:
	protoc --js_out=import_style=commonjs,binary:client definitions.proto