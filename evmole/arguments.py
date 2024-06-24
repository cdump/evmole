from dataclasses import dataclass, field

from .evm.element import Element
from .evm.opcodes import Op
from .evm.stack import StackIndexError
from .evm.vm import UnsupportedOpError, Vm
from .utils import to_bytes


@dataclass(frozen=True)
class Arg:
    offset: int
    path: tuple[int, ...]
    add_val: int
    and_mask: int | None


@dataclass(frozen=True)
class IsZeroResult:
    offset: int
    path: tuple[int, ...]
    add_val: int
    and_mask: int | None


@dataclass(frozen=True)
class InfoValDynamic:
    n_elements: int


@dataclass(frozen=True)
class InfoValArray:
    n_elements: int


@dataclass
class Info:
    tinfo: InfoValDynamic | InfoValArray | None = None
    tname: tuple[str, int] | None = None
    children: dict[int, 'Info'] = field(default_factory=dict)

    def to_str(self, is_root: bool = False) -> str:
        if self.tname is not None:
            name = self.tname[0]
            if name == 'bytes':
                if (
                    self.tinfo is None
                    or (isinstance(self.tinfo, InfoValArray) and self.tinfo.n_elements == 0)
                    or (isinstance(self.tinfo, InfoValDynamic) and self.tinfo.n_elements == 1)
                ):
                    return name
            elif len(self.children) == 0:
                if self.tinfo is None or isinstance(self.tinfo, InfoValDynamic):
                    return name

        start_key = 32 if isinstance(self.tinfo, InfoValArray) else 0
        end_key = max(self.children.keys()) if self.children else 0
        if isinstance(self.tinfo, (InfoValArray, InfoValDynamic)):
            end_key = max(end_key, self.tinfo.n_elements * 32)

        q = []
        for k in range(start_key, end_key + 1, 32):
            q.append(self.children[k].to_str(False) if k in self.children else 'uint256')

        c = f'({",".join(q)})' if len(q) > 1 and not is_root else ','.join(q)

        if isinstance(self.tinfo, InfoValArray):
            return f'{c}[]'

        if isinstance(self.tinfo, InfoValDynamic):
            if end_key == 0 and not self.children:
                return 'bytes'
            if end_key == 32:
                if not self.children:
                    return 'uint256[]'
                if len(self.children) == 1 and next(iter(self.children.values())).tinfo is None:
                    return f'{q[1]}[]'

        return c


class ArgsResult:
    data: Info
    not_bool: set[tuple[int, ...]]

    def __init__(self):
        self.data = Info()
        self.not_bool = set()

    def get_or_create(self, path: tuple[int, ...]):
        node = self.data
        for key in path:
            if key not in node.children:
                node.children[key] = Info()
            node = node.children[key]
        return node

    def get(self, path: tuple[int, ...]):
        node = self.data
        for key in path:
            if key not in node.children:
                return None
            node = node.children[key]
        return node

    def mark_not_bool(self, path: tuple[int, ...], offset: int):
        full_path = (*path, offset)
        el = self.get(full_path)
        if el and el.tname and el.tname[0] == 'bool':
            el.tname = None
        self.not_bool.add(full_path)

    def set_tname(self, path: tuple[int, ...], offset: int | None, tname: str, confidence: int):
        full_path = (*path, offset) if offset is not None else path
        if tname == 'bool' and full_path in self.not_bool:
            return
        el = self.get_or_create(full_path)
        if el.tname is not None and confidence <= el.tname[1]:
            return
        el.tname = (tname, confidence)

    def array_in_path(self, path: tuple[int, ...]):
        ret = []
        el: Info | None = self.data
        for p in path:
            if el is not None:
                el = el.children.get(p)
                ret.append(el and isinstance(el.tinfo, InfoValArray))
            else:
                ret.append(False)
        return ret

    def set_info(self, path: tuple[int, ...], tinfo: InfoValDynamic | InfoValArray):
        if len(path) == 0:  # root
            return
        el = self.get_or_create(path)
        if isinstance(tinfo, InfoValDynamic):
            if isinstance(el.tinfo, InfoValDynamic) and el.tinfo.n_elements > tinfo.n_elements:
                return
            if isinstance(el.tinfo, InfoValArray):
                return
        if isinstance(el.tinfo, InfoValArray) and isinstance(tinfo, InfoValArray):
            if tinfo.n_elements < el.tinfo.n_elements:
                return
        el.tinfo = tinfo

    def join_to_string(self):
        return '' if not self.data.children else self.data.to_str(True)


def and_mask_to_type(mask: int) -> str | None:
    if mask == 0:
        return None
    if (mask & (mask + 1)) == 0:
        # 0x0000ffff
        bl = mask.bit_length()
        if bl % 8 == 0:
            return 'address' if bl == 160 else f'uint{bl}'
    else:
        # 0xffff0000
        m = int.from_bytes(mask.to_bytes(32, 'big'), 'little')
        if (m & (m + 1)) == 0:
            bl = m.bit_length()
            if bl % 8 == 0:
                return f'bytes{bl // 8}'
    return None


def function_arguments(code: bytes | str, selector: bytes | str, gas_limit: int = int(5e4)) -> str:
    bytes_selector = to_bytes(selector)
    vm = Vm(code=to_bytes(code), calldata=Element(data=bytes_selector, label='calldata'))
    gas_used = 0
    inside_function = False
    args = ArgsResult()
    while not vm.stopped:
        try:
            if inside_function:
                # print(vm, '\n', sep='')
                # print(args.data)
                # print(args.join_to_string())
                pass
            ret = vm.step()
            gas_used += ret[1]
            if gas_used > gas_limit:
                # raise Exception(f'gas overflow: {gas_used} > {gas_limit}')
                break
        except (StackIndexError, UnsupportedOpError) as ex:
            _ = ex
            # print(ex)
            break

        if inside_function is False:
            if ret[0] in {Op.EQ, Op.XOR, Op.SUB}:
                p = int.from_bytes(vm.stack.peek().data, 'big')
                if p == (1 if ret[0] == Op.EQ else 0):
                    inside_function = ret[2].data.endswith(bytes_selector)
            continue

        match ret:
            case (Op.CALLDATASIZE, _):
                vm.stack.pop()
                vm.stack.push_uint(131072)

            case ((Op.CALLDATALOAD | Op.CALLDATACOPY) as op, _, Element(Arg(offset, path, add_val, _)), *sa):
                if add_val >= 4 and (add_val - 4) % 32 == 0:
                    full_path = (*path, offset)
                    po = 0
                    if add_val != 4:
                        po += sum(32 if is_arr else 0 for is_arr in args.array_in_path(path))
                        if po > (add_val - 4):
                            po = 0
                    new_off = add_val - 4 - po
                    args.set_info(full_path, InfoValDynamic(new_off // 32))

                    mem_offset = int.from_bytes(sa[0].data, 'big') if op == Op.CALLDATACOPY else 0

                    if new_off == 0 and args.array_in_path(full_path)[-1]:
                        if op == Op.CALLDATALOAD:
                            vm.stack.peek().data = (1).to_bytes(32, 'big')
                        else:
                            m = vm.memory.get(mem_offset)
                            if m is not None:
                                m.data = (1).to_bytes(32, 'big')

                    new_label = Arg(new_off, full_path, 0, None)
                    if op == Op.CALLDATALOAD:
                        vm.stack.peek().label = new_label
                    else:
                        args.set_tname(path, offset, 'bytes', 10)
                        m = vm.memory.get(mem_offset)
                        if m is not None:
                            m.label = new_label

            case ((Op.CALLDATALOAD | Op.CALLDATACOPY) as op, _, Element() as el, *sa):
                off = int.from_bytes(el.data, 'big')
                if 4 <= off < 131072 - 1024:  # -1024: cut 'trustedForwarder'
                    args.get_or_create((off - 4,))
                    new_label = Arg(off - 4, (), 0, None)
                    if op == Op.CALLDATALOAD:
                        vm.stack.peek().label = new_label
                    else:
                        mem_offset = int.from_bytes(sa[0].data, 'big')
                        m = vm.memory.get(mem_offset)
                        if m is not None:
                            m.label = new_label

            case (
                Op.ADD,
                _,
                Element(Arg(f_offset, f_path, f_add_val, f_and_mask)),
                Element(Arg(s_offset, s_path, s_add_val, s_and_mask)),
            ):
                args.mark_not_bool(f_path, f_offset)
                args.mark_not_bool(s_path, s_offset)
                p = vm.stack.peek()
                if len(f_path) > len(s_path):
                    p.label = Arg(f_offset, f_path, f_add_val + s_add_val, f_and_mask)
                else:
                    p.label = Arg(s_offset, s_path, f_add_val + s_add_val, s_and_mask)

            case (
                (Op.ADD, _, Element(Arg(offset, path, add_val, and_mask)) as el, Element() as ot)
                | (Op.ADD, _, Element() as ot, Element(Arg(offset, path, add_val, and_mask)) as el)
            ):
                args.mark_not_bool(path, offset)

                ot_val = int.from_bytes(ot.data, 'big')

                E256M1 = (1 << 256) - 1
                if (
                    offset == 0
                    and add_val == 0
                    and len(path) != 0
                    and int.from_bytes(el.data, 'big') == 0
                    and int.from_bytes(ot.data, 'big') == E256M1
                ):
                    vm.stack.peek().data = (0).to_bytes(32, 'big')

                add = (ot_val + add_val) & E256M1
                if add < (1 << 32):
                    vm.stack.peek().label = Arg(offset, path, add, and_mask)

            case (
                (Op.MUL as op, _, Element(Arg(0, path, 0, _)) as el, Element() as ot)
                | (Op.MUL as op, _, Element() as ot, Element(Arg(0, path, 0, _)) as el)
                | (Op.SHL as op, _, Element() as ot, Element(Arg(0, path, 0, _)) as el)
            ):
                args.mark_not_bool(path, 0)
                if isinstance(ot.label, Arg):
                    args.mark_not_bool(ot.label.path, ot.label.offset)
                if len(path) != 0:
                    mult = int.from_bytes(ot.data, 'big')
                    if op == Op.SHL:
                        mult = 1 << mult

                    if mult == 1:
                        args.set_tname(path, None, 'bytes', 10)
                    elif mult == 2:
                        args.set_tname(path, None, 'string', 20)
                    elif mult % 32 == 0 and 32 <= mult <= 3200:
                        args.set_info(path, InfoValArray(mult // 32))

                        for el in vm.stack.data:
                            if isinstance(el.label, Arg):
                                lab = el.label
                                if lab.offset == 0 and lab.path == path and lab.add_val == 0:
                                    el.data = (1).to_bytes(32, 'big')

                        for el in vm.memory.data:
                            if isinstance(el[1].label, Arg):
                                lab = el[1].label
                                if lab.offset == 0 and lab.path == path and lab.add_val == 0:
                                    el[1].data = (1).to_bytes(32, 'big')

                        vm.stack.peek().data = ot.data  # = mult

            case (
                (Op.GT, _, Element(Arg(0, path, 0, None)), Element() as ot)
                | (Op.LT, _, Element() as ot, Element(Arg(0, path, 0, None)))
            ):
                args.mark_not_bool(path, 0)
                v = int.from_bytes(ot.data, 'big')
                if v == 0 or v == 31:
                    vm.stack.peek().data = (1).to_bytes(32, 'big')

            case (
                (Op.LT | Op.GT | Op.MUL, _, Element(Arg(offset, path, _, _)), _)
                | (Op.LT | Op.GT | Op.MUL, _, _, Element(Arg(offset, path, _, _)))
            ):
                args.mark_not_bool(path, offset)

            case (
                (Op.AND, _, Element(Arg(offset, path, add_val, None)), Element() as ot)
                | (Op.AND, _, Element() as ot, Element(Arg(offset, path, add_val, None)))
            ):
                args.mark_not_bool(path, offset)
                mask = int.from_bytes(ot.data, 'big')
                t = and_mask_to_type(mask)
                if t is not None:
                    args.set_tname(path, offset, t, 5)
                    vm.stack.peek().label = Arg(offset, path, add_val, mask)

            case (
                (Op.EQ, _, Element(Arg(offset, path, add_val, None)), Element(Arg(s_offset, s_path, s_add_val, mask)))
                | (Op.EQ, _, Element(Arg(s_offset, s_path, s_add_val, mask)), Element(Arg(offset, path, add_val, None)))
            ):
                if s_offset == offset and s_path == path and s_add_val == add_val and mask is not None:
                    t = and_mask_to_type(mask)
                    if t is not None:
                        args.set_tname(path, offset, t, 20)

            case (Op.ISZERO, _, Element(Arg(offset, path, add_val, and_mask))):
                vm.stack.peek().label = IsZeroResult(offset, path, add_val, and_mask)

            case (Op.ISZERO, _, Element(IsZeroResult(offset, path, _, _))):
                # Detect check for 0 in DIV, it's not bool in that case: ISZERO, ISZERO, PUSH off, JUMPI, JUMPDEST, DIV
                is_bool = True
                op = vm.code[vm.pc]
                if op >= Op.PUSH1 and op <= Op.PUSH4:
                    n = op - Op.PUSH0
                    if vm.code[vm.pc + n + 1] == Op.JUMPI:
                        jumpdest = int.from_bytes(vm.code[(vm.pc + 1) : (vm.pc + 1 + n)], signed=False)
                        if jumpdest + 1 < len(vm.code) and vm.code[jumpdest] == Op.JUMPDEST and vm.code[jumpdest + 1] == Op.DIV:
                            is_bool = False
                if is_bool:
                    args.set_tname(path, offset, 'bool', 5)

            case (Op.SIGNEXTEND, _, s0, Element(Arg(offset, path, _, _))):
                if s0 < 32:
                    args.set_tname(path, offset, f'int{(s0+1)*8}', 20)

            case (Op.BYTE, _, _, Element(Arg(offset, path, add_val, and_mask))):
                args.set_tname(path, offset, 'bytes32', 4)

    return args.join_to_string()
