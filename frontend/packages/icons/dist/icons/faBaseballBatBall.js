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

// src/node_modules/@fortawesome/free-solid-svg-icons/faBaseballBatBall.js
var require_faBaseballBatBall = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faBaseballBatBall.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "baseball-bat-ball";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f432";
    var svgPathData = "M464.1 544a80 80 0 1 1 0-160 80 80 0 1 1 0 160zm8.4-576c15.1 0 29.7 5.8 40.7 16.2l47.2 44.8c12.6 12 19.1 29 17.5 46.3-1.2 13.3-7 25.8-16.5 35.2L363.1 309c-7.3 7.3-15.7 13.4-25 18L206 393c-9.2 4.6-17.7 10.7-25 18l-92 92 1.7 1.8c7.7 9.4 7.1 23.3-1.7 32.1s-22.7 9.3-32.1 1.7l-1.8-1.7-48-48-1.7-1.8c-7.7-9.4-7.1-23.3 1.7-32.1s22.7-9.3 32.1-1.7l1.8 1.7 92-92c7.3-7.3 13.4-15.7 18-25l66.1-132.1c4.6-9.2 10.7-17.7 18-25L430.8-14.7C441.9-25.8 456.9-32 472.5-32z";
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
    exports.faBaseballBatBall = exports.definition;
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

// src/_entries/faBaseballBatBall.js
var import_faBaseballBatBall = __toESM(require_faBaseballBatBall());
var export_faBaseballBatBall = import_faBaseballBatBall.definition;
export {
  export_faBaseballBatBall as faBaseballBatBall
};
