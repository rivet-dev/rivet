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

// src/node_modules/@fortawesome/free-brands-svg-icons/faSquareWebAwesome.js
var require_faSquareWebAwesome = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faSquareWebAwesome.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "square-web-awesome";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e683";
    var svgPathData = "M64 32C28.7 32 0 60.7 0 96L0 416c0 35.3 28.7 64 64 64l320 0c35.3 0 64-28.7 64-64l0-320c0-35.3-28.7-64-64-64L64 32zM235 177.6l53 46.4 52.2-10.4c-2.6-3.9-4.2-8.5-4.2-13.6 0-13.3 10.7-24 24-24s24 10.7 24 24c0 13-10.3 23.6-23.2 24L304.5 349.1c-5.2 11.5-16.6 18.9-29.2 18.9l-102.6 0c-12.6 0-24-7.4-29.2-18.9L87.2 224c-12.9-.4-23.2-11-23.2-24 0-13.3 10.7-24 24-24s24 10.7 24 24c0 5-1.5 9.7-4.2 13.6L160 224 213.1 177.6c-8.9-4.1-15-13.1-15-23.6 0-14.4 11.6-26 26-26s26 11.6 26 26c0 10.5-6.2 19.5-15.1 23.6z";
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
    exports.faSquareWebAwesome = exports.definition;
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

// src/_entries/faSquareWebAwesome.js
var import_faSquareWebAwesome = __toESM(require_faSquareWebAwesome());
var export_faSquareWebAwesome = import_faSquareWebAwesome.definition;
export {
  export_faSquareWebAwesome as faSquareWebAwesome
};
