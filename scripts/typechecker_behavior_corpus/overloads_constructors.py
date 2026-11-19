"""Overload selection and constructor diagnostic probes."""

from dataclasses import dataclass
from typing import Generic, Literal, NamedTuple, TypeVar, overload

# fmt: off


@overload
def parse(value: bytes) -> int: ...


@overload
def parse(value: str) -> float: ...


def parse(value: bytes | str) -> int | float:
    return 0


parse(1)  # benchmark: overloads/error | quality: topic=overloads; category=overload; span=1; actual-type=int/Literal[1]; expected-type=bytes,str; name=value; related-candidates=true; hint=candidate-overloads


@overload
def combine(left: int, right: int) -> int: ...


@overload
def combine(left: str, right: str) -> str: ...


def combine(left: object, right: object) -> object:
    return left


combine(1, "bad pair")  # benchmark: overloads/error | quality: topic=overloads; category=overload; span="bad pair"; actual-type=str/Literal['bad pair']; expected-type=int,str; name=right; related-candidates=true; hint=candidate-overloads


@overload
def mode(value: Literal["read"]) -> bytes: ...


@overload
def mode(value: Literal["write"]) -> str: ...


def mode(value: str) -> bytes | str:
    return value


mode("other")  # benchmark: overloads/error | quality: topic=overloads; category=overload; span="other"; actual-type=str/Literal['other']; expected-type=Literal['read'],Literal['write']; name=value; related-candidates=true; hint=candidate-overloads


@overload
def optional_overload(value: int, flag: bool = False) -> int: ...


@overload
def optional_overload(value: str, flag: bool = False) -> str: ...


def optional_overload(value: object, flag: bool = False) -> object:
    return value


optional_overload(object())  # benchmark: overloads/error | quality: topic=overloads; category=overload; span=object(); actual-type=object; expected-type=int,str; name=value; related-candidates=true; hint=candidate-overloads


@overload
def tagged(*, kind: Literal["int"], value: int) -> int: ...


@overload
def tagged(*, kind: Literal["str"], value: str) -> str: ...


def tagged(*, kind: str, value: object) -> object:
    return value


tagged(kind="int", value="bad tagged value")  # benchmark: overloads/error | quality: topic=overloads; category=overload; span="bad tagged value"; actual-type=str/Literal['bad tagged value']; expected-type=int; name=value; related-candidates=true; hint=candidate-overloads


class User:
    def __init__(self, name: str, age: int) -> None:
        self.name = name
        self.age = age


User("name", "bad age")  # benchmark: errors/error | quality: topic=constructors; category=constructor; span="bad age"; actual-type=str/Literal['bad age']; expected-type=int; name=age; hint=assignable


@dataclass
class Point:
    x: int
    y: int


Point("bad x", 2)  # benchmark: errors/error | quality: topic=constructors; category=constructor; span="bad x"; actual-type=str/Literal['bad x']; expected-type=int; name=x; hint=assignable


class Pair(NamedTuple):
    left: int
    right: str


Pair(1, 2)  # benchmark: errors/error | quality: topic=constructors; category=constructor; span=2; actual-type=int/Literal[2]; expected-type=str; name=right; hint=assignable

T = TypeVar("T")


class Holder(Generic[T]):
    def __init__(self, value: T, fallback: T) -> None:
        self.value = value
        self.fallback = fallback


bad_holder: Holder[int] = Holder[int](1, "bad fallback")  # benchmark: errors/error | quality: topic=constructors; category=constructor; span="bad fallback"; actual-type=str/Literal['bad fallback']; expected-type=int; name=fallback; hint=assignable


class KeywordConstructed:
    def __init__(self, *, count: int) -> None:
        self.count = count


KeywordConstructed(count="bad count")  # benchmark: errors/error | quality: topic=constructors; category=constructor; span="bad count"; actual-type=str/Literal['bad count']; expected-type=int; name=count; hint=assignable

# fmt: on
