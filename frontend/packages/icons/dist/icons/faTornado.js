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

// src/node_modules/@fortawesome/free-solid-svg-icons/faTornado.js
var require_faTornado = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faTornado.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "tornado";
    var width = 448;
    var height = 512;
    var aliases = [127786];
    var unicode = "f76f";
    var svgPathData = "M0 32L0 45.6C0 62.7 1.7 79.6 5 96l352.8 0c3.2-6.9 7.5-13.3 13-18.8l38.6-38.6c4.2-4.2 6.6-10 6.6-16 0-12.5-10.1-22.6-22.6-22.6L32 0C14.3 0 0 14.3 0 32zM355.7 144l-336 0c12.4 29.7 30.2 56.8 52.7 80l339.6 0-47.2-62.9c-3.9-5.2-7-11-9.2-17.1zM242.6 334.7c8.2 4.8 15.8 10.7 22.5 17.3L445 352c2-9.8 3-19.9 3-30.1 0-17.1-2.9-34-8.6-49.9L136 272 242.6 334.7zM294.3 400c5.3 17 6.5 35.2 3.6 53l-5.9 35.7c-2 12.2 7.4 23.4 19.8 23.4 5.3 0 10.4-2.1 14.2-5.9l78.2-78.2c8.5-8.5 15.8-17.8 21.9-27.9l-131.8 0z";
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
    exports.faTornado = exports.definition;
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

// src/_entries/faTornado.js
var import_faTornado = __toESM(require_faTornado());
var export_faTornado = import_faTornado.definition;
export {
  export_faTornado as faTornado
};
