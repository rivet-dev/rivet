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

// src/node_modules/@fortawesome/free-solid-svg-icons/faIceCream.js
var require_faIceCream = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faIceCream.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "ice-cream";
    var width = 384;
    var height = 512;
    var aliases = [127848];
    var unicode = "f810";
    var svgPathData = "M335.1 160c.6-5.3 .9-10.6 .9-16 0-79.5-64.5-144-144-144S48 64.5 48 144c0 5.4 .3 10.7 .9 16l-.9 0c-26.5 0-48 21.5-48 48s21.5 48 48 48l288 0c26.5 0 48-21.5 48-48s-21.5-48-48-48l-.9 0zM64 304L169.2 529.5c4.1 8.8 13 14.5 22.8 14.5s18.6-5.7 22.8-14.5L320 304 64 304z";
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
    exports.faIceCream = exports.definition;
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

// src/_entries/faIceCream.js
var import_faIceCream = __toESM(require_faIceCream());
var export_faIceCream = import_faIceCream.definition;
export {
  export_faIceCream as faIceCream
};
