"""Protocol compatibility and decorator diagnostic probes."""

from collections.abc import Callable, Iterator
from typing import ParamSpec, Protocol, TypeVar

# fmt: off


class Reader(Protocol):
    def read(self, size: int) -> bytes: ...


class MissingRead:
    pass


reader: Reader = MissingRead()  # benchmark: errors/error | quality: topic=protocols; category=protocol; span=MissingRead(); actual-type=MissingRead; expected-type=Reader; name=read; hint=missing-member


class WrongRead:
    def read(self, size: str) -> str:
        return size


wrong_reader: Reader = WrongRead()  # benchmark: errors/error | quality: topic=protocols; category=protocol; span=WrongRead(); actual-type=WrongRead; expected-type=Reader; name=read; hint=assignable


class SizedIterable(Protocol):
    def __len__(self) -> int: ...
    def __iter__(self) -> Iterator[int]: ...


class OnlySized:
    def __len__(self) -> int:
        return 0


sized_iterable: SizedIterable = OnlySized()  # benchmark: errors/error | quality: topic=protocols; category=protocol; span=OnlySized(); actual-type=OnlySized; expected-type=SizedIterable; name=__iter__; hint=missing-member


class Writable(Protocol):
    value: int


class TextValue:
    value = "text"


writable: Writable = TextValue()  # benchmark: errors/error | quality: topic=protocols; category=protocol; span=TextValue(); actual-type=TextValue; expected-type=Writable; name=value; hint=assignable


class CallbackProtocol(Protocol):
    def __call__(self, value: int, *, flag: bool) -> str: ...


def callback_impl(value: str, *, flag: bool) -> int:
    return len(value)


callback_protocol: CallbackProtocol = callback_impl  # benchmark: errors/error | quality: topic=protocols; category=protocol; span=callback_impl; actual-type=callback_impl/Callable; expected-type=CallbackProtocol; name=value; hint=assignable

P = ParamSpec("P")
R = TypeVar("R")


def preserve(func: Callable[P, R]) -> Callable[P, R]:
    return func


@preserve
def decorated(value: int) -> str:
    return str(value)


decorated("bad decorated argument")  # benchmark: errors/error | quality: topic=decorators; category=call; span="bad decorated argument"; actual-type=str/Literal['bad decorated argument']; expected-type=int; name=value; hint=assignable


def int_result(func: Callable[[], int]) -> Callable[[], int]:
    return func


@int_result  # benchmark: errors/error | quality: topic=decorators; category=decorator; span=int_result; actual-type=str; expected-type=int; name=func; hint=assignable
def returns_text() -> str:
    return "text"


def needs_string_function(func: Callable[[str], str]) -> Callable[[str], str]:
    return func


@needs_string_function  # benchmark: errors/error | quality: topic=decorators; category=decorator; span=needs_string_function; actual-type=int; expected-type=str; name=func; hint=assignable
def takes_number(value: int) -> int:
    return value


def decorate_method(func: Callable[[object, int], str]) -> Callable[[object, int], str]:
    return func


class DecoratedMethods:
    @decorate_method  # benchmark: errors/error | quality: topic=decorators; category=decorator; span=decorate_method; actual-type=str; expected-type=int; name=func; hint=assignable
    def wrong_method(self, value: str) -> int:
        return len(value)


def decorator_factory(prefix: str) -> Callable[[Callable[[], int]], Callable[[], int]]:
    def decorate(func: Callable[[], int]) -> Callable[[], int]:
        return func
    return decorate


@decorator_factory(1)  # benchmark: errors/error | quality: topic=decorators; category=call; span=1; actual-type=int/Literal[1]; expected-type=str; name=prefix; hint=assignable
def factory_decorated() -> int:
    return 1

# fmt: on
