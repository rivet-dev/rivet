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

// src/node_modules/@fortawesome/free-solid-svg-icons/faPenRuler.js
var require_faPenRuler = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faPenRuler.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "pen-ruler";
    var width = 512;
    var height = 512;
    var aliases = ["pencil-ruler"];
    var unicode = "f5ae";
    var svgPathData = "M404 0c19.2 0 37.6 7.6 51.1 21.2l35.7 35.7C504.4 70.4 512 88.8 512 108s-7.6 37.6-21.2 51.1L445.9 204 308 66.1 352.9 21.2C366.4 7.6 384.8 0 404 0zM58.9 315.1L274.1 100 412 237.9 196.9 453.1c-10.7 10.7-24.1 18.5-38.7 22.6L30.4 511.1c-8.3 2.3-17.3 0-23.4-6.2s-8.5-15.1-6.2-23.4L36.4 353.8c4.1-14.6 11.8-27.9 22.6-38.7zM225.4 80.8L80.8 225.4 11.7 156.3c-15.6-15.6-15.6-40.9 0-56.6l88-88c15.6-15.6 40.9-15.6 56.6 0l5.9 5.9-56.3 56.3c-7.8 7.8-7.8 20.5 0 28.3s20.5 7.8 28.3 0l56.3-56.3 34.9 34.9zM431.2 286.6l34.9 34.9-56.3 56.3c-7.8 7.8-7.8 20.5 0 28.3s20.5 7.8 28.3 0l56.3-56.3 5.9 5.9c15.6 15.6 15.6 40.9 0 56.6l-88 88c-15.6 15.6-40.9 15.6-56.6 0l-69.1-69.1 144.6-144.6z";
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
    exports.faPenRuler = exports.definition;
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

// src/_entries/faPenRuler.js
var import_faPenRuler = __toESM(require_faPenRuler());
var export_faPenRuler = import_faPenRuler.definition;
export {
  export_faPenRuler as faPenRuler
};
