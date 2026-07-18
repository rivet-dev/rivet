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

// src/node_modules/@fortawesome/free-brands-svg-icons/faCentercode.js
var require_faCentercode = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faCentercode.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "centercode";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f380";
    var svgPathData = "M201.8 254.8a64.1 64.1 0 1 1 127.4 13.8 64.1 64.1 0 1 1 -127.4-13.8zm41.6 248.9C96.7 496-8.2 365.5 10.1 224.3 21.3 137.7 75.9 67.4 149.2 32.3 310.2-44.8 498.9 69.7 503.9 248.9 508 395.9 385.5 511.1 243.4 503.7zm179.9-180c27.9-118-160.5-205.9-237.2-234.2-57.5 56.3-69.1 188.6-33.8 344.4 68.8 15.8 169.1-26.4 271-110.2z";
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
    exports.faCentercode = exports.definition;
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

// src/_entries/faCentercode.js
var import_faCentercode = __toESM(require_faCentercode());
var export_faCentercode = import_faCentercode.definition;
export {
  export_faCentercode as faCentercode
};
