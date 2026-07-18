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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCowbell.js
var require_faCowbell = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCowbell.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "cowbell";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f8b3";
    var svgPathData = "M112 96l16 0 0-56c0-22.1 17.9-40 40-40L280 0c22.1 0 40 17.9 40 40l0 56 16 0c21.4 0 40.3 14.2 46.2 34.8l64 224c4.1 14.5 1.2 30.1-7.8 42.1s-23.3 19.1-38.3 19.1L48 416c-15.1 0-29.3-7.1-38.3-19.1s-12-27.6-7.8-42.1l64-224C71.7 110.2 90.6 96 112 96zm160 0l0-48-96 0 0 48 96 0zm16 352c0 35.3-28.7 64-64 64s-64-28.7-64-64l128 0z";
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
    exports.faCowbell = exports.definition;
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

// src/_entries/faCowbell.js
var import_faCowbell = __toESM(require_faCowbell());
var export_faCowbell = import_faCowbell.definition;
export {
  export_faCowbell as faCowbell
};
