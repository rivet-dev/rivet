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

// src/node_modules/@fortawesome/free-solid-svg-icons/faPersonHalfDress.js
var require_faPersonHalfDress = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faPersonHalfDress.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "person-half-dress";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "e548";
    var svgPathData = "M143.4-3.9c9.7-16.8 27.8-28.1 48.6-28.1 30.9 0 56 25.1 56 56 0 25.4-16.9 46.8-40 53.7l0 0c-5.1 1.5-10.4 2.3-16 2.3-30.9 0-56-25.1-56-56l0 0c0-10.1 2.7-19.7 7.4-27.9zM318.3 299.1L272 236.7 272 512c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-160 0-.4 0-238.6c36.2 4.4 69.2 23.4 91.2 53l70.5 95c10.5 14.2 7.6 34.2-6.6 44.8s-34.2 7.6-44.8-6.6zM176 113l0 399c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-128-25.8 0c-10.9 0-18.6-10.7-15.2-21.1l43-129-48.3 65.1c-10.5 14.2-30.6 17.2-44.8 6.6s-17.2-30.6-6.6-44.8l70.5-95c22-29.6 55.1-48.6 91.2-53z";
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
    exports.faPersonHalfDress = exports.definition;
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

// src/_entries/faPersonHalfDress.js
var import_faPersonHalfDress = __toESM(require_faPersonHalfDress());
var export_faPersonHalfDress = import_faPersonHalfDress.definition;
export {
  export_faPersonHalfDress as faPersonHalfDress
};
