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

// src/node_modules/@fortawesome/free-brands-svg-icons/faFonticonsFi.js
var require_faFonticonsFi = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faFonticonsFi.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "fonticons-fi";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "f3a2";
    var svgPathData = "M114.4 224l92.4 0-15.2 51.2-76.4 0 0 157.8c0 8-2.8 9.2 4.4 10l59.6 5.6 0 34.4-179.2 0 0-35.2 29.2-2.8c7.2-.8 9.2-3.2 9.2-10.8l0-155.8c0-3.2-4-3.2-8-3.2l-30.4 0 0-51.2 38.4 0 0-28.8c0-68 36.4-96 106-96 46.8 0 88.8 11.2 88.8 72.4L163.6 180c.4-25.6-6-31.6-22.4-31.6-25.2 0-26 13.6-26 37.6l0 32c0 3.2-4.8 6-.8 6zM384 483l-140.8 0 0-34.4 28-3.6c7.2-.8 10.4-2.4 10.4-10l0-148c0-5.6-4-9.2-9.2-10.8l-33.2-8.8 9.2-40.4 110 0 0 208c0 8-3.6 8.8 4 10l21.6 3.6 0 34.4zM354 135.8l12.4 45.6-10 10-42.8-22.8-42.8 22.8-10-10 12.4-45.6-30-36.4 4.8-10 38 0 21.2-38.4 12.8 0 21.2 38.4 38 0 4.8 13.2-30 33.2z";
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
    exports.faFonticonsFi = exports.definition;
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

// src/_entries/faFonticonsFi.js
var import_faFonticonsFi = __toESM(require_faFonticonsFi());
var export_faFonticonsFi = import_faFonticonsFi.definition;
export {
  export_faFonticonsFi as faFonticonsFi
};
