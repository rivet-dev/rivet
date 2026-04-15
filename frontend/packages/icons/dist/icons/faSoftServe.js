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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSoftServe.js
var require_faSoftServe = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSoftServe.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "soft-serve";
    var width = 384;
    var height = 512;
    var aliases = [127846, "creemee"];
    var unicode = "e400";
    var svgPathData = "M208 0c-6.5 0-12.3 3.9-14.8 9.9s-1.1 12.9 3.5 17.4l6.1 6.1c3.4 3.4 5.3 7.9 5.3 12.7C208 56 200 64 190.1 64L128 64c-35.3 0-64 28.7-64 64c0 11.8 3.2 22.9 8.8 32.4C40.8 164 16 191.1 16 224c0 35.3 28.7 64 64 64l224 0c35.3 0 64-28.7 64-64s-28.7-64-64-64l-2.3 0c11.5-14.4 18.3-32.7 18.3-52.6C320 48.1 271.9 0 212.6 0L208 0zM48 320l0 32c0 17.7 14.3 32 32 32l1.8 0 11.1 99.5c1.8 16.2 15.5 28.5 31.8 28.5l134.7 0c16.3 0 30-12.3 31.8-28.5L302.2 384l1.8 0c17.7 0 32-14.3 32-32l0-32L48 320z";
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
    exports.faSoftServe = exports.definition;
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

// src/_entries/faSoftServe.js
var import_faSoftServe = __toESM(require_faSoftServe());
var export_faSoftServe = import_faSoftServe.definition;
export {
  export_faSoftServe as faSoftServe
};
