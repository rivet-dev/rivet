var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBellOn.js
var require_faBellOn = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBellOn.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bell-on";
    var width = 640;
    var height = 512;
    var aliases = [128365];
    var unicode = "f8fa";
    var svgPathData = "M320 0c-17.7 0-32 14.3-32 32l0 19.2C215 66 160 130.6 160 208l0 18.8c0 47-17.3 92.4-48.5 127.6l-7.4 8.3c-8.4 9.4-10.4 22.9-5.3 34.4S115.4 416 128 416l384 0c12.6 0 24-7.4 29.2-18.9s3.1-25-5.3-34.4l-7.4-8.3C497.3 319.2 480 273.9 480 226.8l0-18.8c0-77.4-55-142-128-156.8L352 32c0-17.7-14.3-32-32-32zm45.3 493.3c12-12 18.7-28.3 18.7-45.3l-64 0-64 0c0 17 6.7 33.3 18.7 45.3s28.3 18.7 45.3 18.7s33.3-6.7 45.3-18.7zM0 200c0 13.3 10.7 24 24 24l80 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-80 0c-13.3 0-24 10.7-24 24zm536-24c-13.3 0-24 10.7-24 24s10.7 24 24 24l80 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-80 0zM597.5 21.3c-5.9-11.9-20.3-16.7-32.2-10.7l-64 32c-11.9 5.9-16.7 20.3-10.7 32.2s20.3 16.7 32.2 10.7l64-32c11.9-5.9 16.7-20.3 10.7-32.2zM53.3 53.5l64 32c11.9 5.9 26.3 1.1 32.2-10.7s1.1-26.3-10.7-32.2l-64-32C62.9 4.6 48.5 9.4 42.5 21.3s-1.1 26.3 10.7 32.2z";
    exports.definition = {
      prefix,
      iconName,
      icon: [
        width,
        height,
        aliases,
        unicode,
        svgPathData
      ]
    };
    exports.faBellOn = exports.definition;
    exports.prefix = prefix;
    exports.iconName = iconName;
    exports.width = width;
    exports.height = height;
    exports.ligatures = aliases;
    exports.unicode = unicode;
    exports.svgPathData = svgPathData;
    exports.aliases = aliases;
  }
});

// src/_entries/faBellOn.js
var import_faBellOn = __toESM(require_faBellOn());
var export_faBellOn = import_faBellOn.definition;
export {
  export_faBellOn as faBellOn
};
