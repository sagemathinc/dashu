R = PolynomialRing(QQ, 'x', 7, order='degrevlex')
I = sage.rings.ideal.Cyclic(R, 7)
B = I.groebner_basis()
print(len(B))
