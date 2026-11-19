"""Definitions consumed by cross-file diagnostic probes."""

from typing import Literal, Protocol, TypedDict, overload


class RemoteRecord(TypedDict):
    name: str
    count: int


class RemoteProtocol(Protocol):
    def render(self, width: int) -> str: ...


class RemoteBox:
    def __init__(self, value: int) -> None:
        self.value = value


def remote_call(value: int) -> None:
    pass


@overload
def remote_overload(value: Literal["a"]) -> int: ...


@overload
def remote_overload(value: Literal["b"]) -> str: ...


def remote_overload(value: str) -> int | str:
    return value
