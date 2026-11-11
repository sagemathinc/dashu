# Phase 0, part D: the real workloads profiled natively (python3 -m sagebrush workload.sage under perf).
# Each entry: name -> Sage source. Sizes are chosen to run tens of seconds natively on one thread.
WORKLOADS = {
    # F4 over QQ (the default proof path: multimodular F4, CRT and rational reconstruction, exact check)
    "f4-katsura8-QQ": "R = PolynomialRing(QQ, 'x', 9, order='degrevlex')\nI = sage.rings.ideal.Katsura(R, 9)\nB = I.groebner_basis()\nprint(len(B))\n",
    "f4-cyclic7-QQ": "R = PolynomialRing(QQ, 'x', 7, order='degrevlex')\nI = sage.rings.ideal.Cyclic(R, 7)\nB = I.groebner_basis()\nprint(len(B))\n",
    # modular symbols: an exact Hecke characteristic polynomial (multimodular, CRT)
    "modsym-389-T2": "M = ModularSymbols(Gamma0(389), 2, sign=1)\nf = M.hecke_polynomial(2)\nprint(f.degree())\n",
    # a class group (relations, HNF, regulator)
    "classgroup-quartic": "K.<a> = NumberField(x^4 - 2*x^3 + 17*x - 101)\nprint(K.class_number())\n",
    # integer factorization (ECM-sized factors)
    "factor-ecm": "n = next_prime(10^19) * next_prime(10^21) * next_prime(10^23)\nprint(len(factor(n)))\n",
    # the multiprecision core at high precision (mpmath)
    "mpmath-10k": "import mpmath\nmpmath.mp.dps = 20000\nx = mpmath.pi + mpmath.e\nfor i in range(5):\n    y = mpmath.exp(x) * mpmath.sqrt(x) / mpmath.log(x + i)\nprint(int(y) % 1000)\n",
}
