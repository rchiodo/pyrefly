"""Generic and recursive/nested-container diagnostic probes."""

from typing import Generic, TypeVar

# fmt: off

T = TypeVar("T")
IntT = TypeVar("IntT", bound=int)
NumberT = TypeVar("NumberT", int, float)


def first(values: list[T]) -> T:
    return values[0]


bad_first: int = first(["text"])  # benchmark: errors/error | quality: topic=generics; category=assignment; span=first(["text"]); actual-type=str/Literal['text']; expected-type=int; name=bad_first; hint=assignable


def bounded(value: IntT) -> IntT:
    return value


bounded(1.5)  # benchmark: errors/error | quality: topic=generics; category=generic; span=1.5; actual-type=float/Literal[1.5]; expected-type=int; name=value; hint=assignable


def constrained(left: NumberT, right: NumberT) -> NumberT:
    return left


constrained(1, "text")  # benchmark: errors/error | quality: topic=generics; category=call; span="text"; actual-type=str/Literal['text']; expected-type=int; name=right; hint=assignable


class Store(Generic[T]):
    def __init__(self, value: T) -> None:
        self.value = value


integer_store: Store[int] = Store[int]("text")  # benchmark: errors/error | quality: topic=generics; category=constructor; span="text"; actual-type=str/Literal['text']; expected-type=int; name=value; hint=assignable


integer_store.value = "replacement"  # benchmark: errors/error | quality: topic=generics; category=assignment; span="replacement"; actual-type=str/Literal['replacement']; expected-type=int; name=value; hint=assignable

JsonValue = int | str | list["JsonValue"] | dict[str, "JsonValue"]


nested_list: list[list[int]] = [[1], ["bad nested"]]  # benchmark: errors/error | quality: topic=recursive-containers; category=assignment; span="bad nested"; actual-type=str/Literal['bad nested']; expected-type=int; name=nested_list; hint=assignable
nested_dict: dict[str, list[int]] = {"values": [1, "bad value"]}  # benchmark: errors/error | quality: topic=recursive-containers; category=assignment; span="bad value"; actual-type=str/Literal['bad value']; expected-type=int; name=nested_dict; hint=assignable
recursive_value: JsonValue = {"items": [1, object()]}  # benchmark: errors/error | quality: topic=recursive-containers; category=assignment; span=object(); actual-type=object; expected-type=JsonValue; name=recursive_value; hint=assignable


def append_nested(values: list[list[int]]) -> None:
    values[0].append("bad append")  # benchmark: errors/error | quality: topic=recursive-containers; category=call; span="bad append"; actual-type=str/Literal['bad append']; expected-type=int; name=object; hint=assignable


def update_recursive(value: dict[str, JsonValue]) -> None:
    value["bad"] = {1, 2}  # benchmark: errors/error | quality: topic=recursive-containers; category=call; span={1, 2}; actual-type=set; expected-type=JsonValue; name=value; hint=assignable

# fmt: on
