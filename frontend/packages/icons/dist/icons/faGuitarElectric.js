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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faGuitarElectric.js
var require_faGuitarElectric = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faGuitarElectric.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "guitar-electric";
    var width = 512;
    var height = 512;
    var aliases = [127928];
    var unicode = "f8be";
    var svgPathData = "M352 81.1l0 33.6L243.8 223c-14.6-26.1-5.5-50.4 3.2-64.7c4.8-7.9 4.8-18.7-1.7-25.3s-17.1-6.6-23.6 0l-29.9 30.2c-9.1 9.2-14.7 21.5-20.2 33.5c0 0 0 0 0 0s0 0 0 0c-1.2 2.6-2.3 5.1-3.5 7.6c-3.5 7.3-8.9 15.7-17.1 24c-24.7 25-64.7 40.2-89.6 47.7c-14.5 4.4-28.1 11.7-38.7 22.4c-30 30.4-30 79.6 0 110l79.9 80.8c30 30.4 78.7 30.4 108.8 0c10.7-10.8 17.9-24.5 22.2-39.2c7.5-25.2 22.5-65.6 47.2-90.6c10-10.1 20-16 28.3-19.3l1.1-.5s0 0 0 0c11.3-4.6 23-9.4 31.6-18l21.6-21.8c6.5-6.6 6.5-17.3 0-23.8s-17.3-6.6-25.8-3c-11.1 4.7-28 7.3-48.3-4.7L461.3 96l2.7 0c26.5 0 48-21.5 48-48s-21.5-48-48-48l-1.1 0c-9.7 0-19.2 2.9-27.3 8.3L366.2 54.5C357.3 60.4 352 70.4 352 81.1zM195.3 268.7l48 48c6.2 6.2 6.2 16.4 0 22.6s-16.4 6.2-22.6 0l-48-48c-6.2-6.2-6.2-16.4 0-22.6s16.4-6.2 22.6 0zm-64 64l48 48c6.2 6.2 6.2 16.4 0 22.6s-16.4 6.2-22.6 0l-48-48c-6.2-6.2-6.2-16.4 0-22.6s16.4-6.2 22.6 0z";
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
    exports.faGuitarElectric = exports.definition;
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

// src/_entries/faGuitarElectric.js
var import_faGuitarElectric = __toESM(require_faGuitarElectric());
var export_faGuitarElectric = import_faGuitarElectric.definition;
export {
  export_faGuitarElectric as faGuitarElectric
};
