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

// src/node_modules/@fortawesome/free-solid-svg-icons/faStroopwafel.js
var require_faStroopwafel = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faStroopwafel.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "stroopwafel";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f551";
    var svgPathData = "M0 256a256 256 0 1 1 512 0 256 256 0 1 1 -512 0zM312.6 63.7c-6.2-6.2-16.4-6.2-22.6 0L256 97.6 222.1 63.7c-6.2-6.2-16.4-6.2-22.6 0s-6.2 16.4 0 22.6l33.9 33.9-45.3 45.3-56.6-56.6c-6.2-6.2-16.4-6.2-22.6 0s-6.2 16.4 0 22.6l56.6 56.6-45.3 45.3-33.9-33.9c-6.2-6.2-16.4-6.2-22.6 0s-6.2 16.4 0 22.6L97.6 256 63.7 289.9c-6.2 6.2-6.2 16.4 0 22.6s16.4 6.2 22.6 0l33.9-33.9 45.3 45.3-56.6 56.6c-6.2 6.2-6.2 16.4 0 22.6s16.4 6.2 22.6 0l56.6-56.6 45.3 45.3-33.9 33.9c-6.2 6.2-6.2 16.4 0 22.6s16.4 6.2 22.6 0l33.9-33.9 33.9 33.9c6.2 6.2 16.4 6.2 22.6 0s6.2-16.4 0-22.6l-33.9-33.9 45.3-45.3 56.6 56.6c6.2 6.2 16.4 6.2 22.6 0s6.2-16.4 0-22.6l-56.6-56.6 45.3-45.3 33.9 33.9c6.2 6.2 16.4 6.2 22.6 0s6.2-16.4 0-22.6l-33.9-33.9 33.9-33.9c6.2-6.2 6.2-16.4 0-22.6s-16.4-6.2-22.6 0l-33.9 33.9-45.3-45.3 56.6-56.6c6.2-6.2 6.2-16.4 0-22.6s-16.4-6.2-22.6 0l-56.6 56.6-45.3-45.3 33.9-33.9c6.2-6.2 6.2-16.4 0-22.6zM142.9 256l45.3-45.3 45.3 45.3-45.3 45.3-45.3-45.3zm67.9 67.9l45.3-45.3 45.3 45.3-45.3 45.3-45.3-45.3zM278.6 256l45.3-45.3 45.3 45.3-45.3 45.3-45.3-45.3zm22.6-67.9l-45.3 45.3-45.3-45.3 45.3-45.3 45.3 45.3z";
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
    exports.faStroopwafel = exports.definition;
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

// src/_entries/faStroopwafel.js
var import_faStroopwafel = __toESM(require_faStroopwafel());
var export_faStroopwafel = import_faStroopwafel.definition;
export {
  export_faStroopwafel as faStroopwafel
};
