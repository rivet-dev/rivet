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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSirenOn.js
var require_faSirenOn = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSirenOn.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "siren-on";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e02e";
    var svgPathData = "M69.3 36l48 32c11 7.4 14 22.3 6.7 33.3s-22.3 14-33.3 6.7l-48-32c-11-7.4-14-22.3-6.7-33.3s22.3-14 33.3-6.7zM597.3 76l-48 32c-11 7.4-25.9 4.4-33.3-6.7s-4.4-25.9 6.7-33.3l48-32c11-7.4 25.9-4.4 33.3 6.7s4.4 25.9-6.7 33.3zM24 192l64 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-64 0c-13.3 0-24-10.7-24-24s10.7-24 24-24zm528 0l64 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-64 0c-13.3 0-24-10.7-24-24s10.7-24 24-24zM144 352l25-199.9c4-32 31.2-56.1 63.5-56.1l175 0c32.3 0 59.5 24 63.5 56.1L496 352l-261.9 0 21.8-174c1.1-8.8-5.1-16.8-13.9-17.9s-16.8 5.1-17.9 13.9L201.9 352 144 352zM96 416c0-17.7 14.3-32 32-32l384 0c17.7 0 32 14.3 32 32l0 32c0 17.7-14.3 32-32 32l-384 0c-17.7 0-32-14.3-32-32l0-32z";
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
    exports.faSirenOn = exports.definition;
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

// src/_entries/faSirenOn.js
var import_faSirenOn = __toESM(require_faSirenOn());
var export_faSirenOn = import_faSirenOn.definition;
export {
  export_faSirenOn as faSirenOn
};
