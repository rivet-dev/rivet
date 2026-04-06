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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceGlasses.js
var require_faFaceGlasses = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceGlasses.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-glasses";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e377";
    var svgPathData = "M512 256c0 141.4-114.6 256-256 256S0 397.4 0 256c0-22.1 2.8-43.5 8.1-64L48 192l0 56c0 30.9 25.1 56 56 56l80 0c30.9 0 56-25.1 56-56l0-56 32 0 0 56c0 30.9 25.1 56 56 56l80 0c30.9 0 56-25.1 56-56l0-56 39.9 0c5.3 20.5 8.1 41.9 8.1 64zM256 0C363.4 0 455.4 66.2 493.4 160l-34.8 0c-9-18.9-28.3-32-50.6-32l-80 0c-22.3 0-41.6 13.1-50.6 32l-42.8 0c-9-18.9-28.3-32-50.6-32l-80 0c-22.3 0-41.6 13.1-50.6 32l-34.8 0C56.6 66.2 148.6 0 256 0zM80 192l0-8c0-13.3 10.7-24 24-24l80 0c13.3 0 24 10.7 24 24l0 64c0 13.3-10.7 24-24 24l-80 0c-13.3 0-24-10.7-24-24l0-56zm61.5 147.9c-6.7 5.8-7.4 15.9-1.6 22.6c22.2 25.7 61 53.5 116.1 53.5s93.8-27.9 116.1-53.5c5.8-6.7 5.1-16.8-1.6-22.6s-16.8-5.1-22.6 1.6C330 362.2 299.4 384 256 384s-74-21.8-91.9-42.5c-5.8-6.7-15.9-7.4-22.6-1.6zM304 184c0-13.3 10.7-24 24-24l80 0c13.3 0 24 10.7 24 24l0 8 0 56c0 13.3-10.7 24-24 24l-80 0c-13.3 0-24-10.7-24-24l0-64zM184.4 224a24 24 0 1 0 -48 0 24 24 0 1 0 48 0zm168 24a24 24 0 1 0 0-48 24 24 0 1 0 0 48z";
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
    exports.faFaceGlasses = exports.definition;
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

// src/_entries/faFaceGlasses.js
var import_faFaceGlasses = __toESM(require_faFaceGlasses());
var export_faFaceGlasses = import_faFaceGlasses.definition;
export {
  export_faFaceGlasses as faFaceGlasses
};
