M = ModularSymbols(Gamma0(10007), 2, sign=1)
f = M.hecke_polynomial(2)
print(f.degree())
