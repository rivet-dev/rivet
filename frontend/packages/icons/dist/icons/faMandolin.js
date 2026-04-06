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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faMandolin.js
var require_faMandolin = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faMandolin.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "mandolin";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f6f9";
    var svgPathData = "M435.3 3.6C444.8-2.3 457.1-.9 465 7l40 40c7.9 7.9 9.3 20.2 3.4 29.7l-40 64c-3.2 5.1-8.2 8.8-14 10.4L388 169.2l-65.9 65.9C323.7 322.4 311.5 512 176 512C78.8 512 0 433.2 0 336C0 200.5 189.6 188.3 276.9 189.8L342.8 124l18.1-66.3c1.6-5.8 5.3-10.8 10.4-14l64-40zM208 352a48 48 0 1 0 0-96 48 48 0 1 0 0 96z";
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
    exports.faMandolin = exports.definition;
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

// src/_entries/faMandolin.js
var import_faMandolin = __toESM(require_faMandolin());
var export_faMandolin = import_faMandolin.definition;
export {
  export_faMandolin as faMandolin
};
