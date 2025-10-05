package main

import (
	"example.com/internal/zu/engine/core"
	zu_mod "example.com/internal/zu/engine/zu-mod"
	"go.bytecodealliance.org/cm"
)

func init() {
	// Реализуем экспортируемые функции из WIT
	zu_mod.Exports.Init = func() {
		core.Info("Engine module initialized ✅")

	}

	zu_mod.Exports.Update = func() cm.Result[string, struct{}, string] {
		core.Debug("Engine update tick...")
		// Возвращаем пустой успешный результат (без ошибки)
		return cm.Result[string, struct{}, string]{}
	}
}

func main() {}
