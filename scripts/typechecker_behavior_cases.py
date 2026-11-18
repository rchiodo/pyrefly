from dataclasses import dataclass
from typing import (
    Any,
    Generic,
    Literal,
    TypeGuard,
    TypeIs,
    TypeVar,
    TypedDict,
    overload,
)


def takes_int(value: int) -> None:
    pass


good_assignment: int = 1  # benchmark: errors/accept
bad_assignment: int = "not an int"  # benchmark: errors/error
takes_int(1)  # benchmark: errors/accept
takes_int("not an int")  # benchmark: errors/error


def bad_return() -> int:
    return "not an int"  # benchmark: errors/error


class HasMember:
    def present(self) -> int:
        return 1


HasMember().present()  # benchmark: errors/accept
HasMember().missing()  # benchmark: errors/error

T = TypeVar("T")


def identity(value: T) -> T:
    return value


precise_tuple = identity(("tag", 4))
ExactTuple = tuple[Literal["tag"], Literal[4]]
WrongTuple = tuple[Literal["other"], Literal[4]]
precise_tuple_ok: ExactTuple = precise_tuple  # benchmark: specificity/accept
precise_tuple_bad: WrongTuple = precise_tuple  # benchmark: specificity/error


@dataclass
class Box(Generic[T]):
    value: T


mutable_box = Box(("tag", 15))
mutable_box.value = ("other", 16)  # comparison: specificity/accept
mutable_box.value = (1, 16)  # benchmark: specificity/error


def conditional_literal(flag: bool) -> None:
    candidate: int | None = 7 if flag else None
    if candidate is not None:
        value = candidate
    else:
        value = 0
    exact: Literal[7, 0] = value  # benchmark: specificity/accept
    wrong: Literal[8, 0] = value  # benchmark: specificity/error


def union_members(value: int | str) -> None:
    if isinstance(value, str):
        value.upper()  # benchmark: members/accept
    value.upper()  # benchmark: members/error


JsonValue = int | str | list["JsonValue"]


def recursive_members(value: JsonValue) -> None:
    if isinstance(value, list):
        value.append([1, "nested"])  # benchmark: members/accept
        value.append(object())  # benchmark: members/error


class Record(TypedDict):
    name: str
    age: int


record: Record = {"name": "name", "age": 1}
key_pair = identity(("name", 1))
record[key_pair[0]].upper()  # benchmark: members/accept
record[key_pair[1]]  # benchmark: members/error

# Regression source: microsoft/pyright#7586.
LiteralKey = Literal["abc"]
literal_dict: dict[LiteralKey, None] = {}
literal_comprehension = [value for value in literal_dict]
literal_comprehension_ok: list[LiteralKey] = literal_comprehension  # comparison: specificity/accept
literal_comprehension_bad: list[Literal["other"]] = literal_comprehension  # benchmark: specificity/error


# Regression source: microsoft/pyright#11575.
class JsonSchema(TypedDict, total=False):
    anyOf: list["JsonSchema"]
    oneOf: list["JsonSchema"]


def guarded_typed_dict_key(schema: JsonSchema) -> None:
    for keyword in ("anyOf", "oneOf"):
        if keyword in schema:
            len(schema[keyword])  # benchmark: members/accept
    schema["missing"]  # benchmark: members/error


def is_str_guard(value: str | None) -> TypeGuard[str]:
    return value is not None


def is_str_is(value: str | None) -> TypeIs[str]:
    return value is not None


def need_str(value: str) -> None:
    pass


# Regression source: pyrefly-org/pyrefly#5263.
def keyword_type_guard(value: str | None) -> None:
    if is_str_guard(value=value):
        need_str(value)  # benchmark: members/accept


def keyword_type_is(value: str | None) -> None:
    if is_str_is(value=value):
        need_str(value)  # benchmark: members/accept


# Regression source: pyrefly-org/pyrefly#5405.
class OpenRecord(TypedDict):
    x: int


class TargetRecord(TypedDict):
    x: int
    y: int


def unpack_missing_key(value: OpenRecord) -> TargetRecord:
    return {**value}  # benchmark: errors/error


@overload
def convert(value: Literal[0]) -> Literal["zero"]: ...


@overload
def convert(value: int) -> str: ...


def convert(value: int) -> str:
    return str(value)


literal_pair = identity((0, "tag"))
Zero = Literal["zero"]
One = Literal["one"]
zero: Zero = convert(literal_pair[0])  # benchmark: overloads/accept
not_one: One = convert(literal_pair[0])  # benchmark: overloads/error


class IntResult:
    def int_only(self) -> int:
        return 1


class StrResult:
    def str_only(self) -> str:
        return ""


@overload
def ambiguous(value: list[int]) -> IntResult: ...


@overload
def ambiguous(value: list[str]) -> StrResult: ...


def ambiguous(value: Any) -> Any:
    return IntResult()


def ambiguous_members(source: list[Any]) -> None:
    result = ambiguous(source)
    result.int_only()  # benchmark: overloads/accept
    result.str_only()  # benchmark: overloads/accept
    result.missing()  # benchmark: overloads/error


@overload
def defaulted(value: list[int], second: int = 0) -> list[int]: ...


@overload
def defaulted(value: list[str], second: int = 0) -> list[str]: ...


def defaulted(value: Any, second: int = 0) -> Any:
    return value


def defaulted_members(source: list[Any]) -> None:
    result = defaulted(source)
    result.append(1)  # benchmark: overloads/accept
    result.append("text")  # benchmark: overloads/accept
    result.append(object())  # benchmark: overloads/error
