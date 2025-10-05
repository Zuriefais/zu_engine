package zumod

import (
	"go.bytecodealliance.org/cm"
)

// ExportsStruct описывает функции, экспортируемые миром "zu:engine/zu-mod@0.1.0".
type ExportsStruct struct {
	Init   func()
	Update func() cm.Result[string, struct{}, string]
}

// Exports должен быть установлен из пользовательского кода (см. init в main.go).
var Exports ExportsStruct
