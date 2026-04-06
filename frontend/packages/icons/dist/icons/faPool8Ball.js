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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPool8Ball.js
var require_faPool8Ball = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPool8Ball.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "pool-8-ball";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e3c5";
    var svgPathData = "M0 256a256 256 0 1 1 512 0A256 256 0 1 1 0 256zm384-64a128 128 0 1 0 -256 0 128 128 0 1 0 256 0zm-80-48c0 11.5-4 22-10.7 30.2C304.8 184.5 312 199.4 312 216c0 30.9-25.1 56-56 56s-56-25.1-56-56c0-16.6 7.2-31.5 18.7-41.8C212 166 208 155.5 208 144c0-26.5 21.5-48 48-48s48 21.5 48 48zm-48 16a16 16 0 1 0 0-32 16 16 0 1 0 0 32zm24 56a24 24 0 1 0 -48 0 24 24 0 1 0 48 0z";
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
    exports.faPool8Ball = exports.definition;
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

// src/_entries/faPool8Ball.js
var import_faPool8Ball = __toESM(require_faPool8Ball());
var export_faPool8Ball = import_faPool8Ball.definition;
export {
  export_faPool8Ball as faPool8Ball
};
