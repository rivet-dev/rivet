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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faShishKebab.js
var require_faShishKebab = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faShishKebab.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "shish-kebab";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f821";
    var svgPathData = "M438.6 73.4c-12.5-12.5-32.8-12.5-45.3 0c-11.5 11.5-12.4 29.7-2.7 42.2c11.1 14.2 13.3 37.7-3 53.9L346.2 211 301 165.8l28.4-28.4c-17-35.7-10.8-79.7 18.8-109.3c37.5-37.5 98.3-37.5 135.8 0c31.4 31.4 36.5 79.2 15.3 115.9c-8.8 15.3-28.4 20.5-43.7 11.7s-20.5-28.4-11.7-43.7c7.1-12.2 5.3-28.2-5.1-38.6zM61.8 405L107 450.2 54.6 502.6c-12.5 12.5-32.8 12.5-45.3 0s-12.5-32.8 0-45.3L61.8 405zM244.3 154.3L357.7 267.7c15.6 15.6 15.6 40.9 0 56.6l-25.4 25.4c-15.6 15.6-40.9 15.6-56.6 0L162.3 236.3c-15.6-15.6-15.6-40.9 0-56.6l25.4-25.4c15.6-15.6 40.9-15.6 56.6 0zM75.7 266.3c15.6-15.6 40.9-15.6 56.6 0L245.7 379.7c15.6 15.6 15.6 40.9 0 56.6l-25.4 25.4c-15.6 15.6-40.9 15.6-56.6 0L50.3 348.3c-15.6-15.6-15.6-40.9 0-56.6l25.4-25.4z";
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
    exports.faShishKebab = exports.definition;
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

// src/_entries/faShishKebab.js
var import_faShishKebab = __toESM(require_faShishKebab());
var export_faShishKebab = import_faShishKebab.definition;
export {
  export_faShishKebab as faShishKebab
};
