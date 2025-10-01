"use client";
import * as Clerk from "@clerk/elements/common";
import * as ClerkSignUp from "@clerk/elements/sign-up";
import { faGithub, faGoogle, faSpinnerThird, Icon } from "@rivet-gg/icons";
import { Link } from "@tanstack/react-router";
import { motion } from "framer-motion";
import { cn } from "@/components";
import { Button } from "@/components/ui/button";
import {
	Card,
	CardContent,
	CardDescription,
	CardFooter,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

export function SignUp() {
	return (
		<motion.div
			className="grid w-full grow items-center px-4 sm:justify-center"
			initial={{ opacity: 0, y: 10 }}
			animate={{ opacity: 1, y: 0 }}
			exit={{ opacity: 0, y: 10 }}
		>
			<ClerkSignUp.Root routing="virtual">
				<Clerk.Loading>
					{(isGlobalLoading) => (
						<>
							<ClerkSignUp.Step name="start">
								<Card className="w-full sm:w-96">
									<CardHeader>
										<CardTitle>Welcome!</CardTitle>
										<CardDescription>
											Enter your email below to login to
											your account.
										</CardDescription>
									</CardHeader>
									<CardContent className="grid gap-y-4">
										<div className="grid grid-cols-2 gap-x-4">
											<Clerk.Connection
												name="github"
												asChild
											>
												<Button
													variant="outline"
													type="button"
													disabled={isGlobalLoading}
												>
													<Clerk.Loading scope="provider:github">
														{(isLoading) =>
															isLoading ? (
																<Icon
																	icon={
																		faSpinnerThird
																	}
																	className="size-4 animate-spin"
																/>
															) : (
																<>
																	<Icon
																		icon={
																			faGithub
																		}
																		className="mr-2 size-4"
																	/>
																	GitHub
																</>
															)
														}
													</Clerk.Loading>
												</Button>
											</Clerk.Connection>
											<Clerk.Connection
												name="google"
												asChild
											>
												<Button
													variant="outline"
													type="button"
													disabled={isGlobalLoading}
												>
													<Clerk.Loading scope="provider:google">
														{(isLoading) =>
															isLoading ? (
																<Icon
																	icon={
																		faSpinnerThird
																	}
																	className="size-4 animate-spin"
																/>
															) : (
																<>
																	<Icon
																		icon={
																			faGoogle
																		}
																		className="mr-2 size-4"
																	/>
																	Google
																</>
															)
														}
													</Clerk.Loading>
												</Button>
											</Clerk.Connection>
										</div>
										<p className="flex items-center gap-x-3 text-sm text-muted-foreground before:h-px before:flex-1 before:bg-border after:h-px after:flex-1 after:bg-border">
											or
										</p>
										<Clerk.Field
											name="emailAddress"
											className="space-y-2"
										>
											<Clerk.Label asChild>
												<Label>Email address</Label>
											</Clerk.Label>
											<Clerk.Input
												type="email"
												required
												placeholder="you@company.com"
												asChild
											>
												<Input />
											</Clerk.Input>
											<Clerk.FieldError className="block text-sm text-destructive" />
										</Clerk.Field>
										<Clerk.Field
											name="password"
											className="space-y-2"
										>
											<Clerk.Label asChild>
												<Label>Password</Label>
											</Clerk.Label>
											<Clerk.Input
												type="password"
												required
												placeholder="Your password"
												asChild
											>
												<Input />
											</Clerk.Input>
											<Clerk.FieldError className="block text-sm text-destructive" />
										</Clerk.Field>
									</CardContent>
									<CardFooter>
										<div className="grid w-full gap-y-4">
											<ClerkSignUp.Captcha className="empty:hidden" />
											<ClerkSignUp.Action submit asChild>
												<Button
													disabled={isGlobalLoading}
												>
													<Clerk.Loading>
														{(isLoading) => {
															return isLoading ? (
																<Icon
																	icon={
																		faSpinnerThird
																	}
																	className="size-4 animate-spin"
																/>
															) : (
																"Continue"
															);
														}}
													</Clerk.Loading>
												</Button>
											</ClerkSignUp.Action>
											<Button
												variant="link"
												size="sm"
												asChild
												className="text-primary-foreground"
											>
												<Link to="/">
													Already have an account?
													Sign in
												</Link>
											</Button>
										</div>
									</CardFooter>
								</Card>
							</ClerkSignUp.Step>

							<ClerkSignUp.Step name="continue">
								<Card className="w-full sm:w-96">
									<CardHeader>
										<CardTitle>
											Continue registration
										</CardTitle>
									</CardHeader>
									<CardContent>
										<Clerk.Field
											name="username"
											className="space-y-2"
										>
											<Clerk.Label>
												<Label>Username</Label>
											</Clerk.Label>
											<Clerk.Input
												type="text"
												required
												asChild
											>
												<Input />
											</Clerk.Input>
											<Clerk.FieldError className="block text-sm text-destructive" />
										</Clerk.Field>
									</CardContent>
									<CardFooter>
										<div className="grid w-full gap-y-4">
											<ClerkSignUp.Action submit asChild>
												<Button
													disabled={isGlobalLoading}
												>
													<Clerk.Loading>
														{(isLoading) => {
															return isLoading ? (
																<Icon
																	icon={
																		faSpinnerThird
																	}
																	className="size-4 animate-spin"
																/>
															) : (
																"Continue"
															);
														}}
													</Clerk.Loading>
												</Button>
											</ClerkSignUp.Action>
										</div>
									</CardFooter>
								</Card>
							</ClerkSignUp.Step>

							<ClerkSignUp.Step name="verifications">
								<ClerkSignUp.Strategy name="email_code">
									<Card className="w-full sm:w-96">
										<CardHeader>
											<CardTitle>
												Verify your email
											</CardTitle>
											<CardDescription>
												Use the verification link sent
												to your email address
											</CardDescription>
										</CardHeader>
										<CardContent className="grid gap-y-4">
											<div className="grid items-center justify-center gap-y-2">
												<Clerk.Field
													name="code"
													className="space-y-2"
												>
													<Clerk.Label className="sr-only">
														Email address
													</Clerk.Label>
													<div className="flex justify-center text-center">
														<Clerk.Input
															type="otp"
															className="flex justify-center has-[:disabled]:opacity-50"
															autoSubmit
															render={({
																value,
																status,
															}) => {
																return (
																	<div
																		data-status={
																			status
																		}
																		className={cn(
																			"relative flex size-10 items-center justify-center border-y border-r border-input text-sm transition-all first:rounded-l-md first:border-l last:rounded-r-md",
																			{
																				"z-10 ring-2 ring-ring ring-offset-background":
																					status ===
																						"cursor" ||
																					status ===
																						"selected",
																			},
																		)}
																	>
																		{value}
																		{status ===
																			"cursor" && (
																			<div className="pointer-events-none absolute inset-0 flex items-center justify-center">
																				<div className="animate-caret-blink h-4 w-px bg-foreground duration-1000" />
																			</div>
																		)}
																	</div>
																);
															}}
														/>
													</div>
													<Clerk.FieldError className="block text-center text-sm text-destructive" />
												</Clerk.Field>
												<ClerkSignUp.Action
													asChild
													resend
													className="text-muted-foreground"
													fallback={({
														resendableAfter,
													}) => (
														<Button
															variant="link"
															size="sm"
															disabled
														>
															Didn&apos;t receive
															a code? Resend (
															<span className="tabular-nums">
																{
																	resendableAfter
																}
															</span>
															)
														</Button>
													)}
												>
													<Button
														type="button"
														variant="link"
														size="sm"
													>
														Didn&apos;t receive a
														code? Resend
													</Button>
												</ClerkSignUp.Action>
											</div>
										</CardContent>
										<CardFooter>
											<div className="grid w-full gap-y-4">
												<ClerkSignUp.Action
													submit
													asChild
												>
													<Button
														disabled={
															isGlobalLoading
														}
													>
														<Clerk.Loading>
															{(isLoading) => {
																return isLoading ? (
																	<Icon
																		icon={
																			faSpinnerThird
																		}
																		className="size-4 animate-spin"
																	/>
																) : (
																	"Continue"
																);
															}}
														</Clerk.Loading>
													</Button>
												</ClerkSignUp.Action>
											</div>
										</CardFooter>
									</Card>
								</ClerkSignUp.Strategy>
							</ClerkSignUp.Step>
						</>
					)}
				</Clerk.Loading>
			</ClerkSignUp.Root>
		</motion.div>
	);
}
