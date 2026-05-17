package main

/*
#include <stdlib.h>
*/
import "C"
import (
	"encoding/json"
	"unsafe"
)

type value struct {
	Type  string      `json:"type"`
	Value interface{} `json:"value,omitempty"`
}

func typedError(message string) *C.char {
	out, _ := json.Marshal(value{Type: "string", Value: message})
	return C.CString(string(out))
}

//export add
func add(args *C.char) *C.char {
	var values []value
	if err := json.Unmarshal([]byte(C.GoString(args)), &values); err != nil {
		return typedError("decode error")
	}

	total := int64(0)
	for _, item := range values {
		if item.Type != "int" {
			return typedError("expected int")
		}
		number, ok := item.Value.(float64)
		if !ok {
			return typedError("invalid int")
		}
		total += int64(number)
	}

	out, _ := json.Marshal(value{Type: "int", Value: total})
	return C.CString(string(out))
}

//export describe
func describe(args *C.char) *C.char {
	return C.CString(`{"type":"string","value":"hello from Go native"}`)
}

//export matter_free_string
func matter_free_string(ptr *C.char) {
	C.free(unsafe.Pointer(ptr))
}

func main() {}
