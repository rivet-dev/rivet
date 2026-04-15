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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTransporter1.js
var require_faTransporter1 = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTransporter1.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "transporter-1";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e043";
    var svgPathData = "M256 0a48 48 0 1 1 0 96 48 48 0 1 1 0-96zM128 512c-17.7 0-32-14.3-32-32s14.3-32 32-32l256 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-88 0-80 0-88 0zM383.7 288c-9.3-.1-18.5-4.2-24.7-12l-31-38.7 0 50.7-144 0 0-50.7L153 276c-6.3 7.8-15.4 11.9-24.7 12l-.3 0s0 0 0 0c-7 0-14.1-2.3-20-7c-13.8-11.1-16-31.2-5-45l57.7-72c18.2-22.7 45.8-36 74.9-36l40.7 0c29.1 0 56.7 13.2 74.9 36L409 236c11 13.8 8.8 33.9-5 45c-5.9 4.7-13 7-20 7c0 0 0 0 0 0l-.3 0zm-327 38.6L64 352l25.4 7.3c3.9 1.1 6.6 4.7 6.6 8.7s-2.7 7.6-6.6 8.7L64 384l-7.3 25.4c-1.1 3.9-4.7 6.6-8.7 6.6s-7.6-2.7-8.7-6.6L32 384 6.6 376.7C2.7 375.6 0 372.1 0 368s2.7-7.6 6.6-8.7L32 352l7.3-25.4c1.1-3.9 4.7-6.6 8.7-6.6s7.6 2.7 8.7 6.6zM480 160l25.4 7.3c3.9 1.1 6.6 4.7 6.6 8.7s-2.7 7.6-6.6 8.7L480 192l-7.3 25.4c-1.1 3.9-4.7 6.6-8.7 6.6s-7.6-2.7-8.7-6.6L448 192l-25.4-7.3c-3.9-1.1-6.6-4.7-6.6-8.7s2.7-7.6 6.6-8.7L448 160l7.3-25.4c1.1-3.9 4.7-6.6 8.7-6.6s7.6 2.7 8.7 6.6L480 160zM176 320l160 0c8.8 0 16 7.2 16 16s-7.2 16-16 16l-160 0c-8.8 0-16-7.2-16-16s7.2-16 16-16zm0 64l160 0c8.8 0 16 7.2 16 16s-7.2 16-16 16l-160 0c-8.8 0-16-7.2-16-16s7.2-16 16-16z";
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
    exports.faTransporter1 = exports.definition;
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

// src/_entries/faTransporter1.js
var import_faTransporter1 = __toESM(require_faTransporter1());
var export_faTransporter1 = import_faTransporter1.definition;
export {
  export_faTransporter1 as faTransporter1
};
