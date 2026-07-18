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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faKerning.js
var require_faKerning = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faKerning.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "kerning";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f86f";
    var svgPathData = "M454.3 23.2c4-7.9 .7-17.5-7.2-21.5s-17.5-.7-21.5 7.2l-240 480c-4 7.9-.7 17.5 7.2 21.5s17.5 .7 21.5-7.2l240-480zM60.6 113.7C52.7 97.9 33.5 91.5 17.7 99.4S-4.5 126.5 3.4 142.3l128 256c5.4 10.8 16.5 17.7 28.6 17.7s23.2-6.8 28.6-17.7l128-256c7.9-15.8 1.5-35-14.3-42.9s-35-1.5-42.9 14.3L160 312.4 60.6 113.7zm320 284.6L403.8 352l152.4 0 23.2 46.3c7.9 15.8 27.1 22.2 42.9 14.3s22.2-27.1 14.3-42.9l-128-256C503.2 102.9 492.1 96 480 96s-23.2 6.8-28.6 17.7l-128 256c-7.9 15.8-1.5 35 14.3 42.9s35 1.5 42.9-14.3zM480 199.6L524.2 288l-88.4 0L480 199.6z";
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
    exports.faKerning = exports.definition;
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

// src/_entries/faKerning.js
var import_faKerning = __toESM(require_faKerning());
var export_faKerning = import_faKerning.definition;
export {
  export_faKerning as faKerning
};
