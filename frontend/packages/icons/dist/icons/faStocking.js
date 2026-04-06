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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faStocking.js
var require_faStocking = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faStocking.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "stocking";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "f7d5";
    var svgPathData = "M96 0C78.3 0 64 14.3 64 32l0 32c0 17.7 14.3 32 32 32l256 0c17.7 0 32-14.3 32-32l0-32c0-17.7-14.3-32-32-32L96 0zm0 128l0 110.9c0 10.7-5.3 20.7-14.2 26.6L55.7 282.9C20.9 306.1 0 345.1 0 386.9l0 2.2C0 457 55 512 122.8 512c24.2 0 48-7.2 68.1-20.6L295 422c35.6-23.7 57-63.7 57-106.5L352 128 96 128z";
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
    exports.faStocking = exports.definition;
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

// src/_entries/faStocking.js
var import_faStocking = __toESM(require_faStocking());
var export_faStocking = import_faStocking.definition;
export {
  export_faStocking as faStocking
};
