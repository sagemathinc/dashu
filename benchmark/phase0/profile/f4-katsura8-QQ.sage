R = PolynomialRing(QQ, 'x', 9, order='degrevlex')
I = sage.rings.ideal.Katsura(R, 9)
B = I.groebner_basis()
print(len(B))
