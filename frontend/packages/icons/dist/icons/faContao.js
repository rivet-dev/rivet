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

// src/node_modules/@fortawesome/free-brands-svg-icons/faContao.js
var require_faContao = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faContao.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "contao";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f26d";
    var svgPathData = "M45.4 305c14.4 67.1 26.4 129 68.2 175L34 480c-18.7 0-34-15.2-34-34L0 66C0 47.3 15.2 32 34 32l57.7 0c-13.8 12.6-26.1 27.2-36.9 43.6-45.4 70-27 146.8-9.4 229.4zM478 32l-90.2 0c21.4 21.4 39.2 49.5 52.7 84.1L303.4 145.4c-14.9-29-37.8-53.3-82.6-43.9-24.6 5.3-41 19.3-48.3 34.6-8.8 18.7-13.2 39.8 8.2 140.3 21.1 100.2 33.7 117.7 49.5 131.2 12.9 11.1 33.4 17 58.3 11.7 44.5-9.4 55.7-40.7 57.4-73.2l137.4-29.6c3.2 71.5-18.7 125.2-57.4 163.6l52.1 0c18.7 0 34-15.2 34-34L512 66c0-18.8-15.2-34-34-34z";
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
    exports.faContao = exports.definition;
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

// src/_entries/faContao.js
var import_faContao = __toESM(require_faContao());
var export_faContao = import_faContao.definition;
export {
  export_faContao as faContao
};
