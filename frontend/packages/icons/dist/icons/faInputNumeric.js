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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faInputNumeric.js
var require_faInputNumeric = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faInputNumeric.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "input-numeric";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e1bd";
    var svgPathData = "M576 128l0 256L64 384l0-256 512 0zM64 64C28.7 64 0 92.7 0 128L0 384c0 35.3 28.7 64 64 64l512 0c35.3 0 64-28.7 64-64l0-256c0-35.3-28.7-64-64-64L64 64zm40 120c0 13.3 10.7 24 24 24l8 0 0 96-16 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l80 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-16 0 0-120c0-13.3-10.7-24-24-24l-32 0c-13.3 0-24 10.7-24 24zm190.6 30.4c5.7-8 17.5-8.6 24-1.2c5.2 5.9 5 14.7-.3 20.5l-72 78c-6.5 7-8.2 17.2-4.3 25.9s12.5 14.4 22 14.4l88 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-33.2 0 34.8-37.7c22-23.8 22.4-60.3 1.1-84.7c-26.9-30.7-75.4-28.4-99.2 4.9l-11.1 15.6c-7.7 10.8-5.2 25.8 5.6 33.5s25.8 5.2 33.5-5.6l11.1-15.6z";
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
    exports.faInputNumeric = exports.definition;
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

// src/_entries/faInputNumeric.js
var import_faInputNumeric = __toESM(require_faInputNumeric());
var export_faInputNumeric = import_faInputNumeric.definition;
export {
  export_faInputNumeric as faInputNumeric
};
