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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faRightLeftLarge.js
var require_faRightLeftLarge = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faRightLeftLarge.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "right-left-large";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e5e1";
    var svgPathData = "M455.6 0c19 0 37.2 7.5 50.6 21L625 139.7c9.6 9.6 15 22.7 15 36.3s-5.4 26.7-15 36.3L506.2 331c-13.4 13.4-31.6 21-50.6 21c-37 0-67.4-28-71.2-64H208c-8.8 0-16-7.2-16-16V231.6c0-4.2-3.4-7.6-7.6-7.6c-2 0-3.9 .8-5.4 2.2L69.3 336 179 445.8c1.4 1.4 3.4 2.2 5.4 2.2c4.2 0 7.6-3.4 7.6-7.6V400c0-8.8 7.2-16 16-16H320c17.7 0 32 14.3 32 32s-14.3 32-32 32H255.6c-3.8 36-34.2 64-71.2 64c-19 0-37.2-7.5-50.6-21L15 372.3C5.4 362.7 0 349.6 0 336s5.4-26.7 15-36.3L133.8 181c13.4-13.4 31.6-21 50.6-21c37 0 67.4 28 71.2 64H432c8.8 0 16 7.2 16 16v40.4c0 4.2 3.4 7.6 7.6 7.6c2 0 3.9-.8 5.4-2.2L570.7 176 461 66.2c-1.4-1.4-3.4-2.2-5.4-2.2c-4.2 0-7.6 3.4-7.6 7.6V112c0 8.8-7.2 16-16 16H320c-17.7 0-32-14.3-32-32s14.3-32 32-32h64.4c3.8-36 34.2-64 71.2-64z";
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
    exports.faRightLeftLarge = exports.definition;
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

// src/_entries/faRightLeftLarge.js
var import_faRightLeftLarge = __toESM(require_faRightLeftLarge());
var export_faRightLeftLarge = import_faRightLeftLarge.definition;
export {
  export_faRightLeftLarge as faRightLeftLarge
};
