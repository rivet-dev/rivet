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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTowerControl.js
var require_faTowerControl = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTowerControl.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "tower-control";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e2a2";
    var svgPathData = "M184 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l16 0 0 48-40 0c-17.7 0-32 14.3-32 32l-85.4 0C16 128-3.2 153.4 4.1 179L44.5 320.2C28.5 321.9 16 335.5 16 352c0 17.7 14.3 32 32 32l32 0 0 96c0 17.7 14.3 32 32 32l224 0c17.7 0 32-14.3 32-32l0-96 32 0c17.7 0 32-14.3 32-32c0-16.5-12.5-30.1-28.5-31.8L443.9 179c7.3-25.6-11.9-51-38.5-51L320 128c0-17.7-14.3-32-32-32l-40 0 0-48 16 0c13.3 0 24-10.7 24-24s-10.7-24-24-24L224 0 184 0zM128 192l0 128-17 0c-.1-.3-.1-.5-.2-.8L74.4 192l53.6 0zm48 128l0-128 96 0 0 128-96 0zM320 192l53.6 0L337.2 319.2l-.1 .3-.1 .5-17 0 0-128z";
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
    exports.faTowerControl = exports.definition;
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

// src/_entries/faTowerControl.js
var import_faTowerControl = __toESM(require_faTowerControl());
var export_faTowerControl = import_faTowerControl.definition;
export {
  export_faTowerControl as faTowerControl
};
