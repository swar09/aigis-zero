import { NextResponse } from 'next/server';
import type { NextRequest } from 'next/server';

export function middleware(request: NextRequest) {
  const { pathname } = request.nextUrl;
  const token = request.cookies.get('aigis_auth')?.value;

  const isLoginPage = pathname === '/login';

  // If accessing login with active token, redirect to home
  if (isLoginPage && token) {
    return NextResponse.redirect(new URL('/', request.url));
  }

  // If accessing protected console routes without token, redirect to login
  if (!isLoginPage && !token) {
    const loginUrl = new URL('/login', request.url);
    return NextResponse.redirect(loginUrl);
  }

  return NextResponse.next();
}

export const config = {
  matcher: [
    /*
     * Match all request paths except:
     * - _next/static (static files)
     * - _next/image (image optimization files)
     * - favicon, logo, api, static assets
     */
    '/((?!_next/static|_next/image|favicon|logo|api|site\\.webmanifest|.*\\.(?:svg|png|jpg|jpeg|gif|webp)$).*)',
  ],
};
