from evmole import function_selectors


def test_selectors_empty_code():
    r = function_selectors('')
    assert r == []
