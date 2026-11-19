"""Diagnostics whose expected types and declarations come from another file."""

from typechecker_behavior_corpus.cross_file_defs import (
    RemoteBox,
    RemoteProtocol,
    RemoteRecord,
    remote_call,
    remote_overload,
)

# fmt: off

remote_assignment: RemoteRecord = {"name": "name", "count": "bad count"}  # benchmark: errors/error | quality: topic=cross-file; category=typed-dict; span="bad count"; actual-type=str/Literal['bad count']; expected-type=int; name=count; hint=assignable
remote_call("bad call")  # benchmark: errors/error | quality: topic=cross-file; category=call; span="bad call"; actual-type=str/Literal['bad call']; expected-type=int; name=value; hint=assignable
RemoteBox("bad constructor")  # benchmark: errors/error | quality: topic=cross-file; category=constructor; span="bad constructor"; actual-type=str/Literal['bad constructor']; expected-type=int; name=value; hint=assignable
remote_overload("other")  # benchmark: overloads/error | quality: topic=cross-file; category=overload; span="other"; actual-type=str/Literal['other']; expected-type=Literal['a'],Literal['b']; name=value; hint=candidate-overloads


class RemoteMismatch:
    pass


remote_protocol: RemoteProtocol = RemoteMismatch()  # benchmark: errors/error | quality: topic=cross-file; category=protocol; span=RemoteMismatch(); actual-type=RemoteMismatch; expected-type=RemoteProtocol; name=render; hint=missing-member
remote_record_missing: RemoteRecord = {"name": "name"}  # benchmark: errors/error | quality: topic=cross-file; category=typed-dict; span={"name": "name"}; expected-type=RemoteRecord; name=count; hint=missing-key

# fmt: on
