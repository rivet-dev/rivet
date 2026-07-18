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

// src/node_modules/@fortawesome/free-brands-svg-icons/faSquareFontAwesome.js
var require_faSquareFontAwesome = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faSquareFontAwesome.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "square-font-awesome";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e5ad";
    var svgPathData = "M64 32C28.7 32 0 60.7 0 96L0 416c0 35.3 28.7 64 64 64l320 0c35.3 0 64-28.7 64-64l0-320c0-35.3-28.7-64-64-64L64 32zm90 122c0 9.3-4.8 17.4-12.1 22l188.9 0c7.3 0 13.2 5.9 13.2 13.2 0 1.8-.4 3.7-1.1 5.4L312 264 342.9 333.4c.7 1.7 1.1 3.5 1.1 5.4 0 7.3-5.9 13.2-13.2 13.2l-186.8 0 0 32-32 0 0-209.5c-6.1-4.8-10-12.2-10-20.5 0-14.4 11.6-26 26-26s26 11.6 26 26z";
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
    exports.faSquareFontAwesome = exports.definition;
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

// src/_entries/faSquareFontAwesome.js
var import_faSquareFontAwesome = __toESM(require_faSquareFontAwesome());
var export_faSquareFontAwesome = import_faSquareFontAwesome.definition;
export {
  export_faSquareFontAwesome as faSquareFontAwesome
};
