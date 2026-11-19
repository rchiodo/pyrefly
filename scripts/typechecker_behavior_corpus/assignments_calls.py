"""Assignment, return, and call diagnostic probes."""

from collections.abc import Callable

# fmt: off

tuple_target: tuple[int, str] = ("bad", "ok")  # benchmark: errors/error | quality: topic=assignments; category=assignment; span="bad"; actual-type=str/Literal['bad']; expected-type=int; name=tuple_target; hint=assignable
list_target: list[int] = [1, "bad"]  # benchmark: errors/error | quality: topic=assignments; category=assignment; span="bad"; actual-type=str/Literal['bad']; expected-type=int; name=list_target; hint=assignable
dict_target: dict[str, int] = {"key": "bad"}  # benchmark: errors/error | quality: topic=assignments; category=assignment; span="bad"; actual-type=str/Literal['bad']; expected-type=int; name=dict_target; hint=assignable
optional_target: int | None = "bad"  # benchmark: errors/error | quality: topic=assignments; category=assignment; span="bad"; actual-type=str/Literal['bad']; expected-type=int | None; name=optional_target; hint=assignable


def returns_text() -> int:
    return "bad return"  # benchmark: errors/error | quality: topic=assignments; category=return; span="bad return"; actual-type=str/Literal['bad return']; expected-type=int; hint=assignable


def accepts_pair(count: int, label: str) -> None:
    pass


accepts_pair("bad count", "ok")  # benchmark: errors/error | quality: topic=calls; category=call; span="bad count"; actual-type=str/Literal['bad count']; expected-type=int; name=count; hint=assignable
accepts_pair(1, label=2)  # benchmark: errors/error | quality: topic=calls; category=call; span=2; actual-type=int/Literal[2]; expected-type=str; name=label; hint=assignable


def accepts_numbers(*values: int) -> None:
    pass


accepts_numbers(1, "bad vararg")  # benchmark: errors/error | quality: topic=calls; category=call; span="bad vararg"; actual-type=str/Literal['bad vararg']; expected-type=int; name=values; hint=assignable


def invoke(callback: Callable[[int], str]) -> str:
    return callback(1)


def wrong_callback(value: str) -> int:
    return len(value)


invoke(wrong_callback)  # benchmark: errors/error | quality: topic=calls; category=call; span=wrong_callback; actual-type=wrong_callback/Callable; expected-type=Callable; name=callback; hint=assignable


def keyword_only(*, enabled: bool) -> None:
    pass


keyword_only(enabled="yes")  # benchmark: errors/error | quality: topic=calls; category=call; span="yes"; actual-type=str/Literal['yes']; expected-type=bool; name=enabled; hint=assignable

# fmt: on
