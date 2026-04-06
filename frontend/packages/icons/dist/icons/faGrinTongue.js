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

// src/node_modules/@fortawesome/free-solid-svg-icons/faFaceGrinTongue.js
var require_faFaceGrinTongue = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faFaceGrinTongue.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-grin-tongue";
    var width = 512;
    var height = 512;
    var aliases = [128539, "grin-tongue"];
    var unicode = "f589";
    var svgPathData = "M0 256c0 104.4 62.5 194.2 152.2 234.1-5.3-13-8.2-27.2-8.2-42.1l0-61.7c-16.9-16.5-30.1-36.7-38.3-59.3-4.3-11.8 7.7-21.8 19.6-18.1 39.2 12.2 83.7 19.1 130.7 19.1s91.5-6.9 130.7-19.1c11.9-3.7 23.9 6.3 19.6 18.1-8.3 22.6-21.5 42.8-38.3 59.3l0 61.7c0 14.9-2.9 29.1-8.2 42.1 89.7-39.8 152.2-129.6 152.2-234.1 0-141.4-114.6-256-256-256S0 114.6 0 256zm176-80a32 32 0 1 1 0 64 32 32 0 1 1 0-64zm128 32a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zm16 240l0-45.4c0-14.7-11.9-26.6-26.6-26.6l-2 0c-11.3 0-21.1 7.9-23.6 18.9-2.8 12.6-20.8 12.6-23.6 0-2.5-11.1-12.3-18.9-23.6-18.9l-2 0c-14.7 0-26.6 11.9-26.6 26.6l0 45.4c0 35.3 28.7 64 64 64s64-28.7 64-64z";
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
    exports.faFaceGrinTongue = exports.definition;
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

// src/_entries/faGrinTongue.js
var import_faFaceGrinTongue = __toESM(require_faFaceGrinTongue());
var export_faGrinTongue = import_faFaceGrinTongue.definition;
export {
  export_faGrinTongue as faGrinTongue
};
