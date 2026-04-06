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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faMango.js
var require_faMango = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faMango.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "mango";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e30f";
    var svgPathData = "M344 0l8 0 24 0 8 0 56 0c39.8 0 72 32.2 72 72c0 13.3-10.7 24-24 24l-40 0c-33.4 0-62.8-17-80-42.9l0 49c32.2 8.3 62.6 25 87.8 50.2C493.3 189.7 512 238.9 512 288c0 76.6-57.5 224-224 224c-144 0-244.1-43.3-244.1-43.3C10.3 457.6-7.9 421.4 3.3 387.9C7.4 375.4 15 365 24.7 357.5c34.3-25.7 52.1-57.6 71.2-91.8c20.6-36.8 42.6-76.3 88.4-113.5C221.7 114.7 270.9 96 320 96l0-72c0-13.3 10.7-24 24-24zm88 304c0-8.8-7.2-16-16-16s-16 7.2-16 16c0 53-43 96-96 96c-8.8 0-16 7.2-16 16s7.2 16 16 16c70.7 0 128-57.3 128-128z";
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
    exports.faMango = exports.definition;
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

// src/_entries/faMango.js
var import_faMango = __toESM(require_faMango());
var export_faMango = import_faMango.definition;
export {
  export_faMango as faMango
};
