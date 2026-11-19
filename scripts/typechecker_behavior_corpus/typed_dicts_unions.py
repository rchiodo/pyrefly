"""TypedDict and union diagnostic probes."""

from typing import Literal, NotRequired, TypedDict

# fmt: off


class UserRecord(TypedDict):
    name: str
    age: int


missing_age: UserRecord = {"name": "Ada"}  # benchmark: errors/error | quality: topic=typed-dicts; category=typed-dict; span={"name": "Ada"}; expected-type=UserRecord; name=age; hint=missing-key
extra_key: UserRecord = {"name": "Ada", "age": 1, "extra": True}  # benchmark: errors/error | quality: topic=typed-dicts; category=typed-dict; span="extra"; name=extra
wrong_age: UserRecord = {"name": "Ada", "age": "old"}  # benchmark: errors/error | quality: topic=typed-dicts; category=typed-dict; span="old"; actual-type=str/Literal['old']; expected-type=int; name=age; hint=assignable


class PartialRecord(TypedDict):
    required: int
    optional: NotRequired[str]


missing_required: PartialRecord = {}  # benchmark: errors/error | quality: topic=typed-dicts; category=typed-dict; span={}; expected-type=PartialRecord; name=required; hint=missing-key


def mutate_record(record: UserRecord) -> None:
    record["name"] = 1  # benchmark: errors/error | quality: topic=typed-dicts; category=typed-dict; span=record["name"] = 1; actual-type=int/Literal[1]; expected-type=str; name=name; hint=assignable


Scalar = int | str


scalar: Scalar = object()  # benchmark: errors/error | quality: topic=unions; category=union; span=object(); actual-type=object; expected-type=int | str; name=scalar; hint=assignable


def accepts_scalar(value: Scalar) -> None:
    pass


accepts_scalar(1.5)  # benchmark: errors/error | quality: topic=unions; category=union; span=1.5; actual-type=float/Literal[1.5]; expected-type=int | str; name=value; hint=assignable


class Cat(TypedDict):
    kind: Literal["cat"]
    lives: int


class Dog(TypedDict):
    kind: Literal["dog"]
    breed: str


Pet = Cat | Dog


bad_pet_tag: Pet = {"kind": "bird", "lives": 1}  # benchmark: errors/error | quality: topic=unions; category=union; span="bird"; actual-type=str/Literal['bird']; expected-type=Literal['cat']/Literal['dog']; name=kind; hint=assignable


def union_member(value: int | str) -> None:
    value.append(1)  # benchmark: members/error | quality: topic=unions; category=member; span=append; actual-type=int | str; name=append; hint=missing-member


def narrowed_union(value: bytes | str) -> None:
    if isinstance(value, bytes):
        value.upper()
    else:
        value.hex()  # benchmark: members/error | quality: topic=unions; category=member; span=hex; actual-type=str; name=hex; hint=missing-member

# fmt: on
