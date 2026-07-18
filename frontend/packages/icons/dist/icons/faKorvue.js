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

// src/node_modules/@fortawesome/free-brands-svg-icons/faKorvue.js
var require_faKorvue = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faKorvue.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "korvue";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f42f";
    var svgPathData = "M387.6 34l-327 0C27.9 34 1.1 60.8 1.1 93.5l0 327.1c0 32.6 26.8 59.4 59.5 59.4l327.1 0c33 0 59.5-26.8 59.5-59.5l0-327C447.1 60.8 420.3 34 387.6 34zM88.2 120.8l96 0 0 116 61.8-116 110.9 0-81.2 132-187.5 0 0-132zM250 392.9l-65.7-113.6 0 113.6-96 0 0-130.8 191.5 0 88.6 130.8-118.4 0z";
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
    exports.faKorvue = exports.definition;
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

// src/_entries/faKorvue.js
var import_faKorvue = __toESM(require_faKorvue());
var export_faKorvue = import_faKorvue.definition;
export {
  export_faKorvue as faKorvue
};
